//! Navigation in two dimensions and on a 3D navigation mesh.
//!
//! The navigation-mesh types have the same names as in [`d3`](crate::d3)
//! (`NavMesh`, `NavAgent`, `NavObstacle`): a bare `Mesh` would clash with the
//! render mesh under `use ponko::*;`.
pub use crate::flow::{FlowField, NavGrid};
#[cfg(feature = "navmesh")]
pub use crate::nav3d::{
    NavAgent3D as NavAgent, NavBakeError, NavBakeSettings, NavMesh3D as NavMesh, NavObstacle3D as NavObstacle,
    NavSource, step_navmesh3d,
};
