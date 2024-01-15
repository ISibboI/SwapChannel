use core::marker::PhantomData;
use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static MASTER_KEY_EXISTS: AtomicBool = AtomicBool::new(false);

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
            assert!(MASTER_KEY_EXISTS.swap(false, Ordering::Relaxed));
        }
    }
}

/// The key used for accessing a [DataPointer].
/// Only one can simultaneously exist at any point, and only if there is no channel key.
pub struct DataKey<'master_key> {
    scope: PhantomData<&'master_key mut MasterKey>,
}

/// The key used for accessing a [ChannelPointer].
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

/// A channel used for communication between threads.
/// It holds to instances of `Data`, which can be accessed or swapped.
/// At any time, either references to `Data` can exist, or a swap operation can be performed.
/// This allows to different threads to hold pointers to one of the `Data` fields each,
/// and a third thread to swap the content of these `Data` fields, resulting in inter-thread communication.
///
/// See [Channel::create] for more info.
#[derive(Debug)]
pub struct Channel<Data> {
    data1: Data,
    data2: Data,
}

/// A pointer to a channel.
/// It can only be accessed using a [ChannelKey].
///
/// This type should always be destroyed via the [Channel::destroy] or [ChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct ChannelPointer<Data> {
    channel: Box<Channel<Data>>,
}

/// A pointer to one of the data fields in a channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [Channel::destroy] or [ChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct DataPointer<Data> {
    data: *mut Data,
}

/// An immutable pointer to one of the data fields in a channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [Channel::destroy_immutable] or [ChannelPointer::destroy_immutable] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct ImmutableDataPointer<Data> {
    data: *const Data,
}

impl<Data> Channel<Data> {
    /// Create a channel and hand out three pointers to it.
    /// One [ChannelPointer] used to swap the content of the two `Data` fields,
    /// and two [DataPointer]s, one to each data field.
    pub fn create(
        data1: Data,
        data2: Data,
    ) -> (ChannelPointer<Data>, DataPointer<Data>, DataPointer<Data>) {
        let mut channel_pointer = ChannelPointer {
            channel: Box::new(Channel { data1, data2 }),
        };
        let data_pointer1 = DataPointer {
            data: (&mut channel_pointer.channel.data1) as *mut Data,
        };
        let data_pointer2 = DataPointer {
            data: (&mut channel_pointer.channel.data2) as *mut Data,
        };
        (channel_pointer, data_pointer1, data_pointer2)
    }

    /// Destroys the channel linked with the three pointers (see [Channel::create]).
    ///
    /// **Panics** if not all three pointers point to the same channel.
    pub fn destroy(
        channel_pointer: ChannelPointer<Data>,
        data_pointer1: DataPointer<Data>,
        data_pointer2: DataPointer<Data>,
    ) -> (Data, Data) {
        let ChannelPointer { mut channel } = channel_pointer;
        let channel_data_pointer1 = (&mut channel.data1) as *mut Data;
        let channel_data_pointer2 = (&mut channel.data2) as *mut Data;
        let DataPointer {
            data: data_pointer1,
        } = data_pointer1;
        let DataPointer {
            data: data_pointer2,
        } = data_pointer2;

        assert!(
            (channel_data_pointer1 == data_pointer1 && channel_data_pointer2 == data_pointer2)
                || (channel_data_pointer1 == data_pointer2
                    && channel_data_pointer2 == data_pointer1)
        );

        (channel.data1, channel.data2)
    }

    /// Destroys the channel linked with the pointers (see [Channel::create]).
    ///
    /// **Panics** if not all pointers point to the same channel.
    pub fn destroy_immutable(
        channel_pointer: ChannelPointer<Data>,
        data_pointer1: DataPointer<Data>,
        data_pointer2: impl IntoIterator<Item = ImmutableDataPointer<Data>>,
    ) -> (Data, Data) {
        let ChannelPointer { mut channel } = channel_pointer;
        let channel_data_pointer1 = (&mut channel.data1) as *mut Data;
        let channel_data_pointer2 = (&mut channel.data2) as *mut Data;
        let DataPointer {
            data: data_pointer1,
        } = data_pointer1;

        for data_pointer2 in data_pointer2 {
            let ImmutableDataPointer {
                data: data_pointer2,
            } = data_pointer2;

            assert!(
                (channel_data_pointer1 == data_pointer1
                    && channel_data_pointer2 as *const Data == data_pointer2)
                    || (channel_data_pointer1 as *const Data == data_pointer2
                        && channel_data_pointer2 == data_pointer1)
            );
        }

        (channel.data1, channel.data2)
    }
}

impl<Data: Clone> Channel<Data> {
    /// Create a channel and hand out three pointers to it.
    /// One [ChannelPointer] used to swap the content of the two `Data` fields,
    /// and two [DataPointer]s, one to each data field.
    ///
    /// In this constructor, both `Data` fields are initialised equally from the given `Data`.
    pub fn create_equal(
        data: Data,
    ) -> (ChannelPointer<Data>, DataPointer<Data>, DataPointer<Data>) {
        Self::create(data.clone(), data)
    }
}

impl<Data> ChannelPointer<Data> {
    /// Swap the two `Data` fields in the channel.
    pub fn swap(&mut self, _key: &ChannelKey) {
        let channel: &mut Channel<Data> = &mut self.channel;
        mem::swap(&mut channel.data1, &mut channel.data2);
    }

    /// Shorthand for [Channel::destroy].
    pub fn destroy(
        self,
        data_pointer1: DataPointer<Data>,
        data_pointer2: DataPointer<Data>,
    ) -> (Data, Data) {
        Channel::destroy(self, data_pointer1, data_pointer2)
    }

    /// Shorthand for [Channel::destroy_immutable].
    pub fn destroy_immutable(
        self,
        data_pointer1: DataPointer<Data>,
        data_pointer2: impl IntoIterator<Item = ImmutableDataPointer<Data>>,
    ) -> (Data, Data) {
        Channel::destroy_immutable(self, data_pointer1, data_pointer2)
    }
}

impl<Data> DataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, _key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }

    /// Get a mutable reference to the `Data` field pointed to by this pointer.
    pub fn get_mut(&mut self, _key: &DataKey) -> &mut Data {
        unsafe { &mut *self.data }
    }

    pub fn into_immutable(self) -> ImmutableDataPointer<Data> {
        ImmutableDataPointer {
            data: self.data as *const Data,
        }
    }
}

impl<Data> ImmutableDataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, _key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Channel, MasterKey};

    #[test]
    fn test() {
        let mut master_key = MasterKey::create();
        let (mut channel_pointer, mut data_pointer1, data_pointer2) = Channel::create(0, 0);

        for _ in 0..3 {
            let data_key = master_key.get_data_key();
            let value = *data_pointer1.get(&data_key) * 3 + *data_pointer2.get(&data_key) + 1;
            *data_pointer1.get_mut(&data_key) = value;

            let channel_key = data_key.into_channel_key();
            channel_pointer.swap(&channel_key);
        }

        let (data1, data2) = Channel::destroy(channel_pointer, data_pointer1, data_pointer2);
        assert_eq!(data1, 2);
        assert_eq!(data2, 6);
    }
}
