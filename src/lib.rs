use core::marker::PhantomData;
use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static MASTER_KEY_EXISTS: AtomicBool = AtomicBool::new(false);

pub struct MasterKey {
    /// A meaningless private field to ensure that
    /// this struct can only be constructed via its constructor.
    #[allow(unused)]
    dummy: (),
}

impl MasterKey {
    pub fn create() -> Self {
        // Assert that the master key does not exist
        // and set it as existing.
        assert!(!MASTER_KEY_EXISTS.swap(true, Ordering::Relaxed));

        // Return a new master key.
        Self { dummy: () }
    }

    pub fn get_data_key(&mut self) -> DataKey<'_> {
        DataKey {
            scope: Default::default(),
        }
    }

    pub fn get_channel_key(&mut self) -> ChannelKey<'_> {
        ChannelKey {
            scope: Default::default(),
        }
    }
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        // Assert that the master key exists
        // and set it as not existing.
        assert!(MASTER_KEY_EXISTS.swap(false, Ordering::Relaxed));
    }
}

pub struct DataKey<'master_key> {
    scope: PhantomData<&'master_key mut MasterKey>,
}

pub struct ChannelKey<'master_key> {
    scope: PhantomData<&'master_key mut MasterKey>,
}

pub struct Channel<Data> {
    data1: Data,
    data2: Data,
}

#[must_use]
pub struct ChannelPointer<Data> {
    channel: Box<Channel<Data>>,
}

#[must_use]
pub struct DataPointer<Data> {
    data: *mut Data,
}

impl<Data> Channel<Data> {
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

    pub fn destroy(
        channel_pointer: ChannelPointer<Data>,
        data_pointer1: DataPointer<Data>,
        data_pointer2: DataPointer<Data>,
    ) {
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
        )
    }
}

impl<Data> ChannelPointer<Data> {
    pub fn swap(&mut self, _key: ChannelKey) {
        let channel: &mut Channel<Data> = &mut self.channel;
        mem::swap(&mut channel.data1, &mut channel.data2);
    }
}

impl<Data> DataPointer<Data> {
    pub fn get(&self, _key: DataKey) -> &Data {
        unsafe { &*self.data }
    }

    pub fn get_mut(&mut self, _key: DataKey) -> &mut Data {
        unsafe { &mut *self.data }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Channel, MasterKey};

    #[test]
    fn test() {
        let master_key = MasterKey::create();
        let (channel_pointer, data_pointer1, data_pointer2) = Channel::create((), ());
    }
}
