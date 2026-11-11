//! Filters described at run time, by component id: what code that can't name
//! Rust types (the C and C++ bindings) uses to say what
//! [`With`](crate::With), [`Without`](crate::Without), [`Or`](crate::Or) and
//! tuples of them say.
//!
//! A [`DynFilter`] compiles to the same inline representation the typed
//! filters build, by the same steps, and archetypes are tested with the same
//! matching code. Where the typed path panics (too many components or
//! clauses), compiling returns a [`FilterError`].

use std::fmt;

use super::{Filters, MAX_OR, MAX_OR_TERMS, MAX_WITH};
use crate::storage::ComponentId;

/// The deepest nesting of [`DynFilter::All`] and [`DynFilter::Or`] that
/// compiles.
pub const MAX_FILTER_DEPTH: usize = 16;

/// A filter over component ids, mirroring the typed filters:
///
/// | Typed | Runtime |
/// |---|---|
/// | `With<T>` | `With(id)` |
/// | `Without<T>` | `Without(id)` |
/// | `(A, B)` | `All(vec![a, b])` |
/// | `()` | `All(vec![])`: matches everything |
/// | `Or<(A, B)>` | `Or(vec![a, b])` |
/// | `Or<()>` | `Or(vec![])`: matches nothing |
///
/// The limits are the typed ones: at most 8 `With` and 8 `Without`
/// components at the top, and an `Or` expands to ANDed clauses (4 at most,
/// with 16 component predicates between them; see [`Or`](crate::Or)).
///
/// ```
/// # use ponko_ecs::*;
/// struct Player;
/// struct Enemy;
/// struct Dead;
/// let mut world = World::new();
/// world.spawn((Player,));
/// world.spawn((Enemy, Dead));
/// let (player, enemy, dead) = (world.register::<Player>(), world.register::<Enemy>(), world.register::<Dead>());
///
/// // `(Or<(With<Player>, With<Enemy>)>, Without<Dead>)`
/// let filter = DynFilter::All(vec![
///     DynFilter::Or(vec![DynFilter::With(player), DynFilter::With(enemy)]),
///     DynFilter::Without(dead),
/// ])
/// .compile()
/// .unwrap();
/// let mut seen = 0;
/// world.query_chunks_filtered(ChunkQuery::new(&[]), &filter, |chunk| seen += chunk.entities.len());
/// assert_eq!(seen, 1);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DynFilter {
    /// The entity has the component.
    With(ComponentId),
    /// The entity doesn't have the component.
    Without(ComponentId),
    /// Every filter holds; none at all holds for everything.
    All(Vec<DynFilter>),
    /// At least one filter holds; none at all holds for nothing.
    Or(Vec<DynFilter>),
}

/// Why a [`DynFilter`] didn't compile: the limits of the typed filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterError {
    /// More than 8 different `With` components.
    TooManyWith,
    /// More than 8 different `Without` components.
    TooManyWithout,
    /// More than 4 clauses after expanding the `Or` filters.
    TooManyClauses,
    /// More than 16 component predicates across the expanded clauses.
    TooManyTerms,
    /// Nested deeper than [`MAX_FILTER_DEPTH`].
    TooDeep,
    /// A component number the world doesn't have (only
    /// [`compile_tree`] can report it).
    UnknownComponent(u32),
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterError::TooManyWith => write!(f, "at most {MAX_WITH} `with` filters per query"),
            FilterError::TooManyWithout => write!(f, "at most {MAX_WITH} `without` filters per query"),
            FilterError::TooManyClauses => write!(f, "at most {MAX_OR} `Or` filters per query"),
            FilterError::TooManyTerms => {
                write!(f, "at most {MAX_OR_TERMS} components across a query's `Or` filters")
            }
            FilterError::TooDeep => write!(f, "filters nest at most {MAX_FILTER_DEPTH} levels deep"),
            FilterError::UnknownComponent(_) => write!(f, "a filter's component id is not registered in this world"),
        }
    }
}

impl std::error::Error for FilterError {}

/// A [`DynFilter`] in the form queries match with: inline, so using it
/// allocates nothing. Make one with [`DynFilter::compile`], run it with
/// [`World::query_chunks_filtered`](crate::World::query_chunks_filtered).
#[derive(Clone, Copy)]
pub struct CompiledFilter(Filters);

impl fmt::Debug for CompiledFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CompiledFilter")
    }
}

impl CompiledFilter {
    pub(crate) fn filters(&self) -> &Filters {
        &self.0
    }
}

/// What one node of a filter tree is: a component test, or a list of child
/// nodes (see [`FilterTree`]). Components are numbers, as
/// [`ComponentId::index`] gives them.
#[derive(Clone, Copy, Debug)]
pub enum FilterShape<'a, T> {
    /// The entity has the component.
    With(u32),
    /// The entity doesn't have the component.
    Without(u32),
    /// Every child holds; no children hold for everything.
    All(&'a [T]),
    /// At least one child holds; no children hold for nothing.
    Or(&'a [T]),
}

/// A filter tree that isn't a [`DynFilter`], such as a binding's own
/// representation: [`compile_tree`] compiles it without building a
/// `DynFilter` first, so compiling allocates nothing.
pub trait FilterTree: Sized {
    /// This node, with its children.
    fn shape(&self) -> FilterShape<'_, Self>;
}

impl FilterTree for DynFilter {
    fn shape(&self) -> FilterShape<'_, Self> {
        match self {
            DynFilter::With(id) => FilterShape::With(id.0),
            DynFilter::Without(id) => FilterShape::Without(id.0),
            DynFilter::All(parts) => FilterShape::All(parts),
            DynFilter::Or(arms) => FilterShape::Or(arms),
        }
    }
}

/// Compiles `root` like [`DynFilter::compile`], looking each component
/// number up with `component` (`None`: [`FilterError::UnknownComponent`]).
/// Nothing is allocated, so a binding can do this for every query.
///
/// # Errors
///
/// When a limit is exceeded or a component isn't known; see [`FilterError`].
pub fn compile_tree<T: FilterTree>(
    root: &T,
    component: impl Fn(u32) -> Option<ComponentId>,
) -> Result<CompiledFilter, FilterError> {
    let mut filters = Filters::default();
    add(root, &mut filters, 0, &component)?;
    Ok(CompiledFilter(filters))
}

impl DynFilter {
    /// Expands the filter into its inline form, applying the limits of the
    /// typed filters.
    ///
    /// # Errors
    ///
    /// When a limit is exceeded; see [`FilterError`].
    pub fn compile(&self) -> Result<CompiledFilter, FilterError> {
        compile_tree(self, |index| Some(ComponentId(index)))
    }
}

/// What `QueryFilter::add` does for the typed filter, step by step.
fn add<T: FilterTree>(
    filter: &T,
    into: &mut Filters,
    depth: usize,
    component: &impl Fn(u32) -> Option<ComponentId>,
) -> Result<(), FilterError> {
    if depth > MAX_FILTER_DEPTH {
        return Err(FilterError::TooDeep);
    }
    let lookup = |index: u32| component(index).ok_or(FilterError::UnknownComponent(index));
    match filter.shape() {
        FilterShape::With(index) => {
            let id = lookup(index)?;
            if !into.with[..into.n_with as usize].contains(&id) {
                if into.n_with as usize >= MAX_WITH {
                    return Err(FilterError::TooManyWith);
                }
                into.add_with(id);
            }
        }
        FilterShape::Without(index) => {
            let id = lookup(index)?;
            if !into.without[..into.n_without as usize].contains(&id) {
                if into.n_without as usize >= MAX_WITH {
                    return Err(FilterError::TooManyWithout);
                }
                into.add_without(id);
            }
        }
        FilterShape::All(parts) => {
            for part in parts {
                add(part, into, depth + 1, component)?;
            }
        }
        FilterShape::Or(arms) => {
            let mut union = Filters::never();
            for arm in arms {
                let mut clauses = Filters::default();
                add(arm, &mut clauses, depth + 1, component)?;
                union = disjoin(&union, &into_clauses(&clauses)?)?;
            }
            append_clauses(into, &union)?;
        }
    }
    Ok(())
}

// The functions below are `Filters::{copy_terms, close_or, append_clauses,
// disjoin, into_clauses}` with their assertions turned into errors, in the
// same order of writes, so they fail exactly where the typed path panics.

fn copy_term(out: &mut Filters, from: &Filters, i: usize) -> Result<(), FilterError> {
    if out.n_terms as usize >= MAX_OR_TERMS {
        return Err(FilterError::TooManyTerms);
    }
    out.copy_terms(from, i, i + 1);
    Ok(())
}

fn close(out: &mut Filters) -> Result<(), FilterError> {
    if out.n_any as usize >= MAX_OR {
        return Err(FilterError::TooManyClauses);
    }
    out.close_or();
    Ok(())
}

fn append_clauses(out: &mut Filters, other: &Filters) -> Result<(), FilterError> {
    let mut start = 0;
    for &end in &other.any_end[..other.n_any as usize] {
        for i in start..end as usize {
            copy_term(out, other, i)?;
        }
        close(out)?;
        start = end as usize;
    }
    Ok(())
}

fn disjoin(a: &Filters, b: &Filters) -> Result<Filters, FilterError> {
    let mut out = Filters::default();
    let mut left = 0;
    for &left_end in &a.any_end[..a.n_any as usize] {
        let mut right = 0;
        for &right_end in &b.any_end[..b.n_any as usize] {
            for i in left..left_end as usize {
                copy_term(&mut out, a, i)?;
            }
            for i in right..right_end as usize {
                copy_term(&mut out, b, i)?;
            }
            close(&mut out)?;
            right = right_end as usize;
        }
        left = left_end as usize;
    }
    Ok(out)
}

/// Required predicates (no `Added`/`Changed` here) become singleton clauses.
fn into_clauses(from: &Filters) -> Result<Filters, FilterError> {
    let mut out = Filters::default();
    for &id in &from.with[..from.n_with as usize] {
        if out.n_terms as usize >= MAX_OR_TERMS {
            return Err(FilterError::TooManyTerms);
        }
        out.add_or_term(id, true);
        close(&mut out)?;
    }
    for &id in &from.without[..from.n_without as usize] {
        if out.n_terms as usize >= MAX_OR_TERMS {
            return Err(FilterError::TooManyTerms);
        }
        out.add_or_term(id, false);
        close(&mut out)?;
    }
    append_clauses(&mut out, from)?;
    Ok(out)
}
