use core::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static MASTER_KEY_EXISTS: AtomicBool = AtomicBool::new(false);

pub mod bidirected;
pub mod directed;
pub mod undirected;

/// The master key.
/// Only one instance of this type can exist at any time.
///
/// A master key can be used to derive a [DataKey] or a [ChannelKey],
/// but only at most one derived key can exist simultaneously,
/// and specifically not both a data key and a channel key at the same time.
pub struct MasterKey {
    /// For debug purposes, multiple master keys can be created.
    /// To prevent them from interfering with the "real" master key, we mark them as "unlimited".
    unlimited: bool,
}

impl MasterKey {
    /// Creates a new master key.
    /// If there already is an existing master key, this function **panics**.
    pub fn create() -> Self {
        // Assert that the master key does not exist
        // and set it as existing.
        // Using `Ordering::Relaxed` is fine here, since if the result is `true`,
        // we anyways panic, and if the result is `false`, nothing happens.
        assert!(!MASTER_KEY_EXISTS.swap(true, Ordering::Relaxed));

        // Return a new master key.
        Self { unlimited: false }
    }

    /// Creates a new master key without checking if there already is one.
    ///
    /// # Safety
    ///
    /// This violates the "only one master key" constraint imposed by this crate and
    /// thus may lead to undefined behavior when a channel is accessed by its
    /// channel key and some data key at the same time.
    ///
    /// Use this only for testing and debugging purposes.
    pub unsafe fn create_unlimited() -> Self {
        Self { unlimited: true }
    }

    /// Get a unique data key from this master key.
    /// The data key mutably borrows from the master key, hence there can be no other keys at the same time.
    pub fn get_data_key(&mut self) -> DataKey<'_> {
        DataKey {
            scope: Default::default(),
        }
    }

    /// Get a unique channel key from this master key.
    /// The channel key mutably borrows from the master key, hence there can be no other keys at the same time.
    pub fn get_channel_key(&mut self) -> ChannelKey<'_> {
        ChannelKey {
            scope: Default::default(),
        }
    }
}

impl Drop for MasterKey {
    /// After dropping a master key, a new one can be created again.
    fn drop(&mut self) {
        if !self.unlimited {
            // Assert that the master key exists
            // and set it as not existing.
            // Using `Ordering::Relaxed` is fine here, since if the result is `true`,
            // we anyways panic, and if the result is `false`, nothing happens.
            assert!(MASTER_KEY_EXISTS.swap(false, Ordering::Relaxed));
        }
    }
}

/// The key used for accessing a data pointer, such as a [`ReadOnlyDataPointer`](directed::ReadOnlyDataPointer), a [`WritableDataPointer`](directed::WritableDataPointer), or a [`DataPointer`](undirected::UndirectedDataPointer).
/// Only one can simultaneously exist at any point, and only if there is no channel key.
pub struct DataKey<'master_key> {
    scope: PhantomData<&'master_key mut MasterKey>,
}

/// The key used for accessing a channel pointer, such as a [`DirectedChannelPointer`](directed::DirectedChannelPointer) or an [`UndirectedChannelPointer`](undirected::UndirectedChannelPointer).
/// Only one can simultaneously exist at any point, and only if there is no data key.
pub struct ChannelKey<'master_key> {
    scope: PhantomData<&'master_key mut MasterKey>,
}

impl<'master_key> DataKey<'master_key> {
    /// Convert this data key into a channel key.
    /// This consumes the data key, ensuring that there is never both a channel key and a data key.
    pub fn into_channel_key(self) -> ChannelKey<'master_key> {
        ChannelKey { scope: self.scope }
    }
}

impl<'master_key> ChannelKey<'master_key> {
    /// Convert this channel key into a data key.
    /// This consumes the channel key, ensuring that there is never both a channel key and a data key.
    pub fn into_data_key(self) -> DataKey<'master_key> {
        DataKey { scope: self.scope }
    }
}
