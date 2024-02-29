//! An undirected swap channel.
//! Both instances of the transmitted data are readable and writable,
//! and the data is swapped instead of being sent only in one direction.

use std::mem;

use crate::{ChannelKey, DataKey};

/// An undirected channel used for communication between threads.
/// It holds two instances of `Data`, which can be accessed or swapped.
/// At any time, either references to `Data` can exist, or a swap operation can be performed.
/// This allows to different threads to hold pointers to one of the `Data` fields each,
/// and a third thread to swap the content of these `Data` fields, resulting in inter-thread communication.
///
/// See [UndirectedChannel::create] for more info.
#[derive(Debug)]
pub struct UndirectedChannel<Data> {
    data1: Data,
    data2: Data,
}

/// A pointer to an undirected channel.
/// It can only be accessed using a [ChannelKey].
///
/// This type should always be destroyed via the [UndirectedChannel::destroy] or [UndirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct UndirectedChannelPointer<Data> {
    channel: Box<UndirectedChannel<Data>>,
}

/// A pointer to one of the data fields in an undirected channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [UndirectedChannel::destroy] or [UndirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct UndirectedDataPointer<Data> {
    data: *mut Data,
}

/// An immutable pointer to one of the data fields in an undirected channel.
/// It can only be accessed using a [DataKey].
///
/// This type should always be destroyed via the [UndirectedChannel::destroy_immutable] or [UndirectedChannelPointer::destroy_immutable] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct ImmutableUndirectedDataPointer<Data> {
    data: *const Data,
}

impl<Data> UndirectedChannel<Data> {
    /// Create an undirected channel and hand out three pointers to it.
    /// One [UndirectedChannelPointer] used to swap the content of the two `Data` fields,
    /// and two [UndirectedDataPointer]s, one to each data field.
    pub fn create(
        data1: Data,
        data2: Data,
    ) -> (
        UndirectedChannelPointer<Data>,
        UndirectedDataPointer<Data>,
        UndirectedDataPointer<Data>,
    ) {
        let mut channel_pointer = UndirectedChannelPointer {
            channel: Box::new(UndirectedChannel { data1, data2 }),
        };
        let data_pointer1 = UndirectedDataPointer {
            data: (&mut channel_pointer.channel.data1) as *mut Data,
        };
        let data_pointer2 = UndirectedDataPointer {
            data: (&mut channel_pointer.channel.data2) as *mut Data,
        };
        (channel_pointer, data_pointer1, data_pointer2)
    }

    /// Destroys the undirected channel linked with the three pointers (see [UndirectedChannel::create]).
    ///
    /// **Panics** if not all three pointers point to the same channel.
    pub fn destroy(
        channel_pointer: UndirectedChannelPointer<Data>,
        data_pointer1: UndirectedDataPointer<Data>,
        data_pointer2: UndirectedDataPointer<Data>,
    ) -> (Data, Data) {
        let UndirectedChannelPointer { mut channel } = channel_pointer;
        let channel_data_pointer1 = (&mut channel.data1) as *mut Data;
        let channel_data_pointer2 = (&mut channel.data2) as *mut Data;
        let UndirectedDataPointer {
            data: data_pointer1,
        } = data_pointer1;
        let UndirectedDataPointer {
            data: data_pointer2,
        } = data_pointer2;

        assert!(
            (channel_data_pointer1 == data_pointer1 && channel_data_pointer2 == data_pointer2)
                || (channel_data_pointer1 == data_pointer2
                    && channel_data_pointer2 == data_pointer1)
        );

        (channel.data1, channel.data2)
    }

    /// Destroys the undirected channel linked with the pointers (see [UndirectedChannel::create]).
    ///
    /// **Panics** if not all pointers point to the same channel.
    pub fn destroy_immutable(
        channel_pointer: UndirectedChannelPointer<Data>,
        data_pointer1: UndirectedDataPointer<Data>,
        data_pointer2: impl IntoIterator<Item = ImmutableUndirectedDataPointer<Data>>,
    ) -> (Data, Data) {
        let UndirectedChannelPointer { mut channel } = channel_pointer;
        let channel_data_pointer1 = (&mut channel.data1) as *mut Data;
        let channel_data_pointer2 = (&mut channel.data2) as *mut Data;
        let UndirectedDataPointer {
            data: data_pointer1,
        } = data_pointer1;

        for data_pointer2 in data_pointer2 {
            let ImmutableUndirectedDataPointer {
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

impl<Data: Clone> UndirectedChannel<Data> {
    /// Create an undirected channel and hand out three pointers to it.
    /// One [UndirectedChannelPointer] used to swap the content of the two `Data` fields,
    /// and two [UndirectedDataPointer]s, one to each data field.
    ///
    /// In this constructor, both `Data` fields are initialised equally from the given `Data`.
    pub fn create_equal(
        data: Data,
    ) -> (
        UndirectedChannelPointer<Data>,
        UndirectedDataPointer<Data>,
        UndirectedDataPointer<Data>,
    ) {
        Self::create(data.clone(), data)
    }
}

impl<Data> UndirectedChannelPointer<Data> {
    /// Swap the two `Data` fields in the undirected channel.
    pub fn swap(&mut self, #[allow(unused)] channel_key: &ChannelKey) {
        let channel: &mut UndirectedChannel<Data> = &mut self.channel;
        mem::swap(&mut channel.data1, &mut channel.data2);
    }

    /// Shorthand for [UndirectedChannel::destroy].
    pub fn destroy(
        self,
        data_pointer1: UndirectedDataPointer<Data>,
        data_pointer2: UndirectedDataPointer<Data>,
    ) -> (Data, Data) {
        UndirectedChannel::destroy(self, data_pointer1, data_pointer2)
    }

    /// Shorthand for [UndirectedChannel::destroy_immutable].
    pub fn destroy_immutable(
        self,
        data_pointer1: UndirectedDataPointer<Data>,
        data_pointer2: impl IntoIterator<Item = ImmutableUndirectedDataPointer<Data>>,
    ) -> (Data, Data) {
        UndirectedChannel::destroy_immutable(self, data_pointer1, data_pointer2)
    }
}

impl<Data> UndirectedDataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, #[allow(unused)] data_key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }

    /// Get a mutable reference to the `Data` field pointed to by this pointer.
    pub fn get_mut(&mut self, #[allow(unused)] data_key: &DataKey) -> &mut Data {
        unsafe { &mut *self.data }
    }

    pub fn into_immutable(self) -> ImmutableUndirectedDataPointer<Data> {
        ImmutableUndirectedDataPointer {
            data: self.data as *const Data,
        }
    }
}

impl<Data> ImmutableUndirectedDataPointer<Data> {
    /// Get a reference to the `Data` field pointed to by this pointer.
    pub fn get(&self, #[allow(unused)] data_key: &DataKey) -> &Data {
        unsafe { &*self.data }
    }
}

impl<Data> Clone for ImmutableUndirectedDataPointer<Data> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Data> Copy for ImmutableUndirectedDataPointer<Data> {}

unsafe impl<Data> Send for UndirectedChannelPointer<Data> {}
unsafe impl<Data> Send for UndirectedDataPointer<Data> {}
unsafe impl<Data> Send for ImmutableUndirectedDataPointer<Data> {}

unsafe impl<Data> Sync for UndirectedChannelPointer<Data> {}
unsafe impl<Data> Sync for UndirectedDataPointer<Data> {}
unsafe impl<Data> Sync for ImmutableUndirectedDataPointer<Data> {}

/// Object-safe trait for [`UndirectedChannelPointer`]s.
pub trait UndirectedSwapChannel: Send + Sync {
    /// Perform the [`UndirectedChannelPointer::swap`] operation.
    fn swap(&mut self, channel_key: &ChannelKey);
}

impl<Data> UndirectedSwapChannel for UndirectedChannelPointer<Data> {
    fn swap(&mut self, channel_key: &ChannelKey) {
        UndirectedChannelPointer::swap(self, channel_key);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        undirected::{UndirectedChannel, UndirectedSwapChannel},
        MasterKey,
    };

    #[test]
    fn test() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel_pointer, mut data_pointer1, data_pointer2) =
            UndirectedChannel::create(0, 0);

        for _ in 0..3 {
            let data_key = master_key.get_data_key();
            let value = *data_pointer1.get(&data_key) * 3 + *data_pointer2.get(&data_key) + 1;
            *data_pointer1.get_mut(&data_key) = value;

            let channel_key = data_key.into_channel_key();
            channel_pointer.swap(&channel_key);
        }

        let (data1, data2) =
            UndirectedChannel::destroy(channel_pointer, data_pointer1, data_pointer2);
        assert_eq!(data1, 2);
        assert_eq!(data2, 6);
    }

    #[test]
    fn ensure_channel_is_object_safe() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel, data1, data2) = UndirectedChannel::create(1, 2);
        let dyn_channel: &mut dyn UndirectedSwapChannel = &mut channel;

        dyn_channel.swap(&master_key.get_channel_key());
        assert_eq!(*data1.get(&master_key.get_data_key()), 2);
        assert_eq!(*data2.get(&master_key.get_data_key()), 1);
        UndirectedChannel::destroy(channel, data1, data2);
    }
}
