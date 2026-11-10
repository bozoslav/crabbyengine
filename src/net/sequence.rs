//! Sequence numbers, authority epochs and "latest wins" filtering for
//! state updates sent on an unreliable lane. Pure data structures: nothing
//! here touches the network, and the game keeps its own packet format.

use std::collections::HashMap;
use std::hash::Hash;

use super::PeerId;

/// Signed distance from `b` to `a` on a 16-bit circle: positive when `a` is
/// ahead of `b`, negative when behind, zero when equal. Sequence numbers more
/// than half the range (32768) apart are ambiguous; that exact distance
/// yields `i16::MIN`.
/// ```
/// use ponko::net::seq_diff;
/// assert_eq!(ponko::net::seq_diff(5, 3), 2);
/// assert_eq!(ponko::net::seq_diff(3, 5), -2);
/// assert_eq!(ponko::net::seq_diff(1, 65535), 2); // wrapped
/// ```
pub const fn seq_diff(a: u16, b: u16) -> i16 {
    a.wrapping_sub(b) as i16
}

/// Is `a` newer than `b`, with 16-bit wrap-around? Equal numbers are not
/// newer. Exactly half the range apart (32768) is ambiguous: neither is newer.
/// ```
/// use ponko::net::seq_newer;
/// assert!(ponko::net::seq_newer(2, 1));
/// assert!(!ponko::net::seq_newer(1, 2));
/// assert!(!ponko::net::seq_newer(7, 7));
/// assert!(ponko::net::seq_newer(0, 65535)); // 0 follows 65535
/// assert!(!ponko::net::seq_newer(65535, 0));
/// ```
pub const fn seq_newer(a: u16, b: u16) -> bool {
    seq_diff(a, b) > 0
}

/// A state update's place in time: the owner's `epoch` (bumped on every
/// authority handoff) and a `seq` counter that wraps and restarts per sender.
/// A higher epoch always beats a lower one; within an epoch the newer `seq`
/// wins.
/// ```
/// use ponko::net::UpdateStamp;
/// let old = ponko::net::UpdateStamp { epoch: 1, seq: 65535 };
/// assert!(ponko::net::UpdateStamp { epoch: 1, seq: 0 }.newer_than(old)); // wrapped
/// assert!(ponko::net::UpdateStamp { epoch: 2, seq: 0 }.newer_than(old)); // handoff beats any seq
/// assert!(!ponko::net::UpdateStamp { epoch: 0, seq: 99 }.newer_than(old));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UpdateStamp {
    /// Authority epoch the update was sent under.
    pub epoch: u16,
    /// Sender's wrapping update counter.
    pub seq: u16,
}
impl UpdateStamp {
    /// Is this strictly newer than `other`? Epochs compare with wrap-around
    /// like sequence numbers.
    /// ```
    /// use ponko::net::UpdateStamp;
    /// let a = ponko::net::UpdateStamp { epoch: 0, seq: 10 };
    /// assert!(!a.newer_than(a));
    /// ```
    pub const fn newer_than(self, other: UpdateStamp) -> bool {
        if self.epoch != other.epoch {
            seq_newer(self.epoch, other.epoch)
        } else {
            seq_newer(self.seq, other.seq)
        }
    }
}

/// A "latest wins per entity" filter for unreliable state updates: accepts
/// an update only if it is newer than everything already accepted for the
/// same key, which drops duplicates, reordered packets and updates from a
/// previous authority epoch.
///
/// Remember a key's stamp for as long as updates may still arrive; call
/// [`forget`](Self::forget) when the entity is gone. A key silent for more
/// than half the sequence range (32768 updates) is ambiguous: forget it
/// after a timeout.
///
/// ```
/// use ponko::net::LatestWins;
/// use ponko::net::UpdateStamp;
/// let mut filter = ponko::net::LatestWins::new();
/// assert!(filter.accept(7u64, ponko::net::UpdateStamp { epoch: 0, seq: 1 }));
/// assert!(filter.accept(7, ponko::net::UpdateStamp { epoch: 0, seq: 3 }));
/// assert!(!filter.accept(7, ponko::net::UpdateStamp { epoch: 0, seq: 2 })); // late: dropped
/// assert!(!filter.accept(7, ponko::net::UpdateStamp { epoch: 0, seq: 3 })); // duplicate
/// assert!(filter.accept(8, ponko::net::UpdateStamp { epoch: 0, seq: 2 })); // other entity
/// ```
#[derive(Clone, Debug)]
pub struct LatestWins<K: Hash + Eq> {
    latest: HashMap<K, UpdateStamp>,
}
impl<K: Hash + Eq> Default for LatestWins<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: Hash + Eq> LatestWins<K> {
    /// An empty filter.
    /// ```
    /// let f = ponko::net::LatestWins::<u32>::new();
    /// assert!(f.is_empty());
    /// ```
    pub fn new() -> Self {
        Self { latest: HashMap::new() }
    }

    /// Records `stamp` for `key` and returns true if it is newer than the
    /// last accepted one (or the first for this key); false if the update
    /// should be dropped.
    /// ```
    /// use ponko::net::LatestWins;
    /// use ponko::net::UpdateStamp;
    /// let mut f = ponko::net::LatestWins::new();
    /// assert!(f.accept("crate", ponko::net::UpdateStamp::default()));
    /// assert!(!f.accept("crate", ponko::net::UpdateStamp::default()));
    /// ```
    pub fn accept(&mut self, key: K, stamp: UpdateStamp) -> bool {
        match self.latest.get_mut(&key) {
            Some(last) if stamp.newer_than(*last) => {
                *last = stamp;
                true
            }
            Some(_) => false,
            None => {
                self.latest.insert(key, stamp);
                true
            }
        }
    }

    /// Like [`accept`](Self::accept) for streams without epochs (epoch 0).
    /// ```
    /// let mut f = ponko::net::LatestWins::new();
    /// assert!(f.accept_seq(1u32, 10));
    /// assert!(!f.accept_seq(1, 9));
    /// ```
    pub fn accept_seq(&mut self, key: K, seq: u16) -> bool {
        self.accept(key, UpdateStamp { epoch: 0, seq })
    }

    /// The newest stamp accepted for `key`.
    /// ```
    /// let mut f = ponko::net::LatestWins::new();
    /// f.accept_seq(1u32, 4);
    /// assert_eq!(f.latest(&1).map(|s| s.seq), Some(4));
    /// ```
    pub fn latest(&self, key: &K) -> Option<UpdateStamp> {
        self.latest.get(key).copied()
    }

    /// Forgets `key`: its next update is accepted whatever its stamp.
    /// ```
    /// let mut f = ponko::net::LatestWins::new();
    /// f.accept_seq(1u32, 4);
    /// f.forget(&1);
    /// assert!(f.accept_seq(1, 2));
    /// ```
    pub fn forget(&mut self, key: &K) {
        self.latest.remove(key);
    }

    /// Forgets every key.
    /// ```
    /// let mut f = ponko::net::LatestWins::new();
    /// f.accept_seq(1u32, 4);
    /// f.clear();
    /// assert!(f.is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.latest.clear();
    }

    /// Keeps only the keys for which `keep` is true.
    /// ```
    /// let mut f = ponko::net::LatestWins::new();
    /// f.accept_seq(1u32, 4);
    /// f.accept_seq(2, 4);
    /// f.retain(|k| *k == 2);
    /// assert_eq!(f.len(), 1);
    /// ```
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.latest.retain(|k, _| keep(k));
    }

    /// Number of keys remembered.
    /// ```
    /// assert_eq!(ponko::net::LatestWins::<u8>::new().len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.latest.len()
    }

    /// Whether no keys are remembered.
    /// ```
    /// assert!(ponko::net::LatestWins::<u8>::new().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty()
    }
}

/// What to do with a state update, judged against an [`Authority`].
/// ```
/// use ponko::net::AuthorityCheck;
/// assert_ne!(ponko::net::AuthorityCheck::Accept, ponko::net::AuthorityCheck::Stale);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityCheck {
    /// From the current owner in the current epoch: apply it.
    Accept,
    /// Sent under an older epoch (before a handoff): drop it.
    Stale,
    /// Same epoch but not from the owner: drop it.
    WrongOwner,
    /// Sent under a newer epoch than we know: the handoff announcement hasn't
    /// arrived yet. Drop it, or hold it until [`Authority::adopt`] sees the
    /// announcement.
    Ahead,
}

/// Who owns an entity right now: an owner peer and an epoch counter. A
/// handoff gives the entity to another peer and bumps the epoch; updates
/// stamped with an older epoch are rejected, so a late packet from the
/// previous owner can't undo the handoff.
///
/// The usual flow: whoever decides (often the host) calls
/// [`handoff`](Self::handoff) and sends the new `(owner, epoch)` on a
/// reliable lane; everyone else calls [`adopt`](Self::adopt) with it.
/// Owners stamp every state update with their epoch and receivers run
/// [`check`](Self::check).
///
/// If two peers hand the same entity off at once (both bump the same epoch),
/// [`adopt`](Self::adopt) breaks the tie in favour of the lower owner id, so
/// every peer converges on the same owner.
///
/// ```
/// use ponko::net::Authority;
/// use ponko::net::AuthorityCheck;
/// use ponko::net::PeerId;
/// let host = ponko::net::PeerId::HOST;
/// let guest = ponko::net::PeerId::from_bits(1);
/// let mut car = ponko::net::Authority::new(host);
/// assert_eq!(car.check(host, 0), ponko::net::AuthorityCheck::Accept);
///
/// let epoch = car.handoff(guest); // the guest takes the wheel
/// assert_eq!(epoch, 1);
/// assert_eq!(car.check(host, 0), ponko::net::AuthorityCheck::Stale); // late packet from before
/// assert_eq!(car.check(guest, 1), ponko::net::AuthorityCheck::Accept);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Authority {
    owner: PeerId,
    epoch: u16,
}
impl Authority {
    /// Owned by `owner`, epoch 0.
    /// ```
    /// let a = ponko::net::Authority::new(ponko::net::PeerId::HOST);
    /// assert_eq!(a.epoch(), 0);
    /// ```
    pub const fn new(owner: PeerId) -> Self {
        Self { owner, epoch: 0 }
    }

    /// Owned by `owner` under a known `epoch` (for a late joiner).
    /// ```
    /// let a = ponko::net::Authority::with_epoch(ponko::net::PeerId::HOST, 9);
    /// assert_eq!(a.epoch(), 9);
    /// ```
    pub const fn with_epoch(owner: PeerId, epoch: u16) -> Self {
        Self { owner, epoch }
    }

    /// The peer whose updates are accepted.
    /// ```
    /// let a = ponko::net::Authority::new(ponko::net::PeerId::HOST);
    /// assert_eq!(a.owner(), ponko::net::PeerId::HOST);
    /// ```
    pub const fn owner(&self) -> PeerId {
        self.owner
    }

    /// Counts handoffs (wrapping).
    /// ```
    /// assert_eq!(ponko::net::Authority::new(ponko::net::PeerId::HOST).epoch(), 0);
    /// ```
    pub const fn epoch(&self) -> u16 {
        self.epoch
    }

    /// Gives the entity to `owner` and returns the new epoch to announce.
    /// Handing it to the current owner still bumps the epoch.
    /// ```
    /// use ponko::net::Authority;
    /// use ponko::net::PeerId;
    /// let mut a = ponko::net::Authority::new(ponko::net::PeerId::HOST);
    /// assert_eq!(a.handoff(ponko::net::PeerId::from_bits(2)), 1);
    /// assert_eq!(a.owner(), ponko::net::PeerId::from_bits(2));
    /// ```
    pub fn handoff(&mut self, owner: PeerId) -> u16 {
        self.epoch = self.epoch.wrapping_add(1);
        self.owner = owner;
        self.epoch
    }

    /// Applies a handoff announced by someone else. Returns true if this
    /// changed the record. Older epochs are ignored (a late announcement); the
    /// same epoch with a different owner is a race, won by the lower owner id.
    /// ```
    /// use ponko::net::Authority;
    /// use ponko::net::PeerId;
    /// let (a, b) = (ponko::net::PeerId::from_bits(1), ponko::net::PeerId::from_bits(2));
    /// let mut x = ponko::net::Authority::new(ponko::net::PeerId::HOST);
    /// assert!(x.adopt(b, 1));
    /// assert!(!x.adopt(ponko::net::PeerId::HOST, 0)); // older: ignored
    /// assert!(x.adopt(a, 1)); // simultaneous handoff: lower id wins
    /// assert!(!x.adopt(b, 1));
    /// assert_eq!(x.owner(), a);
    /// ```
    pub fn adopt(&mut self, owner: PeerId, epoch: u16) -> bool {
        let take = if epoch == self.epoch {
            owner < self.owner
        } else {
            seq_newer(epoch, self.epoch)
        };
        if take {
            self.owner = owner;
            self.epoch = epoch;
        }
        take
    }

    /// Judges an update from `from` stamped with `epoch`.
    /// ```
    /// use ponko::net::Authority;
    /// use ponko::net::AuthorityCheck;
    /// use ponko::net::PeerId;
    /// let a = ponko::net::Authority::new(ponko::net::PeerId::HOST);
    /// assert_eq!(a.check(ponko::net::PeerId::from_bits(3), 0), ponko::net::AuthorityCheck::WrongOwner);
    /// assert_eq!(a.check(ponko::net::PeerId::HOST, 1), ponko::net::AuthorityCheck::Ahead);
    /// ```
    pub fn check(&self, from: PeerId, epoch: u16) -> AuthorityCheck {
        if epoch == self.epoch {
            if from == self.owner {
                AuthorityCheck::Accept
            } else {
                AuthorityCheck::WrongOwner
            }
        } else if seq_newer(epoch, self.epoch) {
            AuthorityCheck::Ahead
        } else {
            AuthorityCheck::Stale
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(n: u64) -> PeerId {
        PeerId::from_bits(n)
    }

    #[test]
    fn sequence_comparison_wraps_around() {
        assert!(seq_newer(1, 0));
        assert!(seq_newer(0, 65535));
        assert!(seq_newer(100, 65500)); // 136 apart across the wrap
        assert!(!seq_newer(65500, 100));
        assert!(!seq_newer(5, 5));
        assert_eq!(seq_diff(100, 65500), 136);
        assert_eq!(seq_diff(65500, 100), -136);
    }

    #[test]
    fn sequence_comparison_is_antisymmetric_except_at_half_range() {
        for a in (0..=u16::MAX).step_by(251) {
            for b in (0..=u16::MAX).step_by(257) {
                let (ab, ba) = (seq_newer(a, b), seq_newer(b, a));
                assert!(!(ab && ba), "{a} {b}");
                if a != b && a.wrapping_sub(b) != 0x8000 {
                    assert!(ab || ba, "{a} {b}");
                }
            }
        }
        assert!(!seq_newer(0, 0x8000) && !seq_newer(0x8000, 0));
    }

    #[test]
    fn newer_than_prefers_epoch_then_sequence() {
        let s = |epoch, seq| UpdateStamp { epoch, seq };
        assert!(s(1, 0).newer_than(s(0, 60000)));
        assert!(!s(0, 60000).newer_than(s(1, 0)));
        assert!(s(0, 0).newer_than(s(65535, 65535))); // epoch wrapped too
        assert!(!s(3, 4).newer_than(s(3, 4)));
    }

    #[test]
    fn latest_wins_drops_old_and_duplicate_updates() {
        let mut f = LatestWins::new();
        let accepted: Vec<bool> = [1u16, 3, 2, 3, 4, 1]
            .into_iter()
            .map(|seq| f.accept_seq(9u64, seq))
            .collect();
        assert_eq!(accepted, [true, true, false, false, true, false]);
        assert_eq!(f.latest(&9).unwrap().seq, 4);
    }

    #[test]
    fn latest_wins_survives_sequence_wrap() {
        let mut f = LatestWins::new();
        for seq in 65530..=u16::MAX {
            assert!(f.accept_seq(1u8, seq));
        }
        for seq in 0..10 {
            assert!(f.accept_seq(1, seq), "{seq}");
        }
        assert!(!f.accept_seq(1, 65535), "pre-wrap packet arriving late");
    }

    #[test]
    fn latest_wins_new_epoch_resets_sequence() {
        let mut f = LatestWins::new();
        assert!(f.accept(1u8, UpdateStamp { epoch: 0, seq: 5000 }));
        // The new owner starts counting from zero.
        assert!(f.accept(1, UpdateStamp { epoch: 1, seq: 0 }));
        assert!(
            !f.accept(1, UpdateStamp { epoch: 0, seq: 5001 }),
            "old owner's late packet"
        );
        assert!(f.accept(1, UpdateStamp { epoch: 1, seq: 1 }));
    }

    #[test]
    fn latest_wins_forget_and_retain() {
        let mut f = LatestWins::new();
        for k in 0..5u32 {
            f.accept_seq(k, 10);
        }
        f.retain(|k| k % 2 == 0);
        assert_eq!(f.len(), 3);
        assert!(f.accept_seq(1, 3), "forgotten keys start fresh");
        f.forget(&1);
        assert_eq!(f.latest(&1), None);
        f.clear();
        assert!(f.is_empty());
    }

    #[test]
    fn handoff_rejects_updates_from_the_previous_owner() {
        let mut a = Authority::new(peer(0));
        assert_eq!(a.check(peer(0), 0), AuthorityCheck::Accept);
        assert_eq!(a.handoff(peer(1)), 1);
        assert_eq!(a.check(peer(0), 0), AuthorityCheck::Stale);
        assert_eq!(a.check(peer(1), 0), AuthorityCheck::Stale);
        assert_eq!(a.check(peer(1), 1), AuthorityCheck::Accept);
        assert_eq!(a.check(peer(0), 1), AuthorityCheck::WrongOwner);
        assert_eq!(a.check(peer(1), 2), AuthorityCheck::Ahead);
    }

    #[test]
    fn adopt_ignores_late_announcements_and_applies_newer_ones() {
        let mut a = Authority::new(peer(0));
        assert!(a.adopt(peer(1), 1));
        assert!(a.adopt(peer(2), 2));
        // A reordered announcement of the first handoff arrives late.
        assert!(!a.adopt(peer(1), 1));
        assert_eq!((a.owner(), a.epoch()), (peer(2), 2));
        // Jumping ahead several epochs (missed announcements) is fine.
        assert!(a.adopt(peer(3), 7));
        assert_eq!(a.epoch(), 7);
    }

    #[test]
    fn simultaneous_handoffs_converge_on_the_lower_owner() {
        // Two peers both take over from epoch 0 at the same moment, then see
        // each other's announcement in opposite orders.
        let mut one = Authority::new(peer(0));
        let mut two = Authority::new(peer(0));
        let ea = one.handoff(peer(1));
        let eb = two.handoff(peer(2));
        assert_eq!(ea, eb);
        one.adopt(peer(2), eb);
        two.adopt(peer(1), ea);
        assert_eq!(one, two);
        assert_eq!(one.owner(), peer(1));
        // The loser's updates are now wrong-owner, the winner's accepted.
        assert_eq!(one.check(peer(2), 1), AuthorityCheck::WrongOwner);
        assert_eq!(one.check(peer(1), 1), AuthorityCheck::Accept);
    }

    #[test]
    fn an_update_can_race_ahead_of_its_handoff_announcement() {
        // Unreliable updates from the new owner overtake the reliable
        // announcement: Ahead until the announcement is adopted.
        let mut a = Authority::new(peer(0));
        assert_eq!(a.check(peer(1), 1), AuthorityCheck::Ahead);
        a.adopt(peer(1), 1);
        assert_eq!(a.check(peer(1), 1), AuthorityCheck::Accept);
    }

    #[test]
    fn epoch_wraps_around() {
        let mut a = Authority::with_epoch(peer(0), u16::MAX);
        assert_eq!(a.handoff(peer(1)), 0);
        assert_eq!(a.check(peer(0), u16::MAX), AuthorityCheck::Stale);
        let mut b = Authority::with_epoch(peer(0), u16::MAX);
        assert!(b.adopt(peer(1), 0));
    }
}
