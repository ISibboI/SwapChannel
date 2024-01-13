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
    current: Data,
    previous: Data,
}

pub struct ChannelPointer<Data> {
    channel: Box<Channel<Data>>,
}

pub struct DataPointer<Data> {
    data: *mut Data,
}

impl<Data> Channel<Data> {
    pub fn create(
        current: Data,
        previous: Data,
    ) -> (ChannelPointer<Data>, DataPointer<Data>, DataPointer<Data>) {
        let mut channel_pointer = ChannelPointer {
            channel: Box::new(Channel { current, previous }),
        };
        let current_pointer = DataPointer {
            data: (&mut channel_pointer.channel.current) as *mut Data,
        };
        let previous_pointer = DataPointer {
            data: (&mut channel_pointer.channel.previous) as *mut Data,
        };
        (channel_pointer, current_pointer, previous_pointer)
    }
}

impl<Data> ChannelPointer<Data> {
    pub fn swap(&mut self, _key: ChannelKey) {
        mem::swap(&mut self.channel.current, &mut self.channel.previous);
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
    use crate::MasterKey;

    #[test]
    #[should_panic]
    fn test() {
        let master_key = MasterKey::create();
    }
}
