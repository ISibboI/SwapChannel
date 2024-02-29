//! A bidirected two-phase channel.
//! This is a wrapper around two directed two-phase channels.
//! Each endpoint has an input and an output pointer,
//! where the input of one endpoint is connected to the output of the other endpoint via a directed channel.

use crate::{
    directed::{DirectedChannel, ReadOnlyDataPointer, WritableDataPointer},
    ChannelKey, DataKey,
};

/// A bidirected channel used for communication between threads.
/// It holds two directed channels.
///
/// See [`DirectedChannel`](crate::directed::DirectedChannel) for more info.
#[derive(Debug)]
pub struct BidirectedChannel<Data1, Data2> {
    channel1: DirectedChannel<Data1>,
    channel2: DirectedChannel<Data2>,
}

/// A pointer to a bidirected channel.
/// It can only be accessed using a [ChannelKey].
///
/// This type should always be destroyed via the [BidirectedChannel::destroy] or [BidirectedChannelPointer::destroy] method to ensure soundness (at runtime).
#[derive(Debug)]
#[must_use]
pub struct BidirectedChannelPointer<Data1, Data2> {
    channel: Box<BidirectedChannel<Data1, Data2>>,
}

/// A pair of pointers to the data fields of a bidirected channel.
/// The `input` pointer points to the read-only end of one of the directed channels,
/// and the `output` pointer points to the writable end of the other directed channel.
///
/// This type should always be destroyed via the [BidirectedChannel::destroy] or [BidirectedChannelPointer::destroy] method to ensure soundness (at runtime).
pub struct BidirectedDataPointer<Input, Output> {
    input: ReadOnlyDataPointer<Input>,
    output: WritableDataPointer<Output>,
}

impl<Data1, Data2> BidirectedChannel<Data1, Data2> {
    /// Create a bidirected channel and hand out three pointers to it.
    /// One [`BidirectedChannelPointer`] used to flush (copy) the content of the writable `Data` fields into the read-only data fields,
    /// two [`BidirectedDataPointer`]s used to read from the input end and write from the output end of the corresponding directed channels.
    ///
    /// See [`BidirectedChannelPointer::flush`] for how to exchange information between the [`BidirectedDataPointer`]s.
    pub fn create(
        read_only1: Data1,
        writable1: Data1,
        read_only2: Data2,
        writable2: Data2,
    ) -> (
        BidirectedChannelPointer<Data1, Data2>,
        BidirectedDataPointer<Data1, Data2>,
        BidirectedDataPointer<Data2, Data1>,
    ) {
        let mut channel_pointer = BidirectedChannelPointer {
            channel: Box::new(BidirectedChannel {
                channel1: DirectedChannel {
                    read_only: read_only1,
                    writable: writable1,
                },
                channel2: DirectedChannel {
                    read_only: read_only2,
                    writable: writable2,
                },
            }),
        };
        let input_data_pointer1 = ReadOnlyDataPointer {
            data: (&channel_pointer.channel.channel1.read_only) as *const Data1,
        };
        let output_data_pointer1 = WritableDataPointer {
            data: (&mut channel_pointer.channel.channel2.writable) as *mut Data2,
        };
        let input_data_pointer2 = ReadOnlyDataPointer {
            data: (&channel_pointer.channel.channel2.read_only) as *const Data2,
        };
        let output_data_pointer2 = WritableDataPointer {
            data: (&mut channel_pointer.channel.channel1.writable) as *mut Data1,
        };
        (
            channel_pointer,
            BidirectedDataPointer {
                input: input_data_pointer1,
                output: output_data_pointer1,
            },
            BidirectedDataPointer {
                input: input_data_pointer2,
                output: output_data_pointer2,
            },
        )
    }

    /// Destroys the bidirected channel linked with the given pointers (see [`BidirectedChannel::create`]).
    ///
    /// **Panics** if not all pointers point to the same channel.
    pub fn destroy(
        channel_pointer: BidirectedChannelPointer<Data1, Data2>,
        data_pointer1: BidirectedDataPointer<Data1, Data2>,
        data_pointer2: BidirectedDataPointer<Data2, Data1>,
    ) -> (Data1, Data1, Data2, Data2) {
        let BidirectedChannelPointer { mut channel } = channel_pointer;
        let BidirectedDataPointer {
            input: ReadOnlyDataPointer { data: read_only1 },
            output: WritableDataPointer { data: writable1 },
        } = data_pointer1;
        let BidirectedDataPointer {
            input: ReadOnlyDataPointer { data: read_only2 },
            output: WritableDataPointer { data: writable2 },
        } = data_pointer2;

        let channel1_read_only = &channel.channel1.read_only as *const Data1;
        let channel2_writable = &mut channel.channel1.writable as *mut Data1;
        let channel2_read_only = &channel.channel2.read_only as *const Data2;
        let channel1_writable = &mut channel.channel2.writable as *mut Data2;

        assert_eq!(channel1_read_only, read_only1);
        assert_eq!(channel1_writable, writable1);
        assert_eq!(channel2_read_only, read_only2);
        assert_eq!(channel2_writable, writable2);

        (
            channel.channel1.read_only,
            channel.channel1.writable,
            channel.channel2.read_only,
            channel.channel2.writable,
        )
    }
}

impl<Data1: Clone, Data2: Clone> BidirectedChannel<Data1, Data2> {
    /// In this constructor, in both directed channels, both `Data` fields are initialised equally from the given `Data`.
    ///
    /// See [`DirectedChannel::create`] for more details.
    pub fn create_equal(
        data1: Data1,
        data2: Data2,
    ) -> (
        BidirectedChannelPointer<Data1, Data2>,
        BidirectedDataPointer<Data1, Data2>,
        BidirectedDataPointer<Data2, Data1>,
    ) {
        Self::create(data1.clone(), data1, data2.clone(), data2)
    }
}

impl<Data1: Clone, Data2: Clone> BidirectedChannelPointer<Data1, Data2> {
    /// Clone the writable `Data`s into the read-only `Data`s.
    pub fn flush(&mut self, key: &ChannelKey) {
        DirectedChannel::flush(&mut self.channel.channel1, key);
        self.channel.channel2.flush(key);
    }
}

impl<Data1, Data2> BidirectedChannelPointer<Data1, Data2> {
    /// Shorthand for [BidirectedChannel::destroy].
    pub fn destroy(
        self,
        data_pointer1: BidirectedDataPointer<Data1, Data2>,
        data_pointer2: BidirectedDataPointer<Data2, Data1>,
    ) -> (Data1, Data1, Data2, Data2) {
        BidirectedChannel::destroy(self, data_pointer1, data_pointer2)
    }
}

impl<Input, Output> BidirectedDataPointer<Input, Output> {
    /// Get a reference to the input data field pointed to by this pointer.
    pub fn get_input(&self, data_key: &DataKey) -> &Input {
        self.input.get(data_key)
    }

    /// Get a mutable reference to the output data field pointed to by this pointer.
    pub fn get_output(&mut self, data_key: &DataKey) -> &mut Output {
        self.output.get_mut(data_key)
    }
}

unsafe impl<Data1, Data2> Send for BidirectedChannelPointer<Data1, Data2> {}
unsafe impl<Input, Output> Send for BidirectedDataPointer<Input, Output> {}

unsafe impl<Data1, Data2> Sync for BidirectedChannelPointer<Data1, Data2> {}
unsafe impl<Input, Output> Sync for BidirectedDataPointer<Input, Output> {}

/// Object-safe trait for [`BidirectedChannelPointer`]s.
pub trait IBidirectedChannel: Send + Sync {
    /// Perform the [`BidirectedChannelPointer::flush`] operation.
    fn flush(&mut self, channel_key: &ChannelKey);
}

impl<Data1: Clone, Data2: Clone> IBidirectedChannel for BidirectedChannelPointer<Data1, Data2> {
    fn flush(&mut self, channel_key: &ChannelKey) {
        BidirectedChannelPointer::flush(self, channel_key);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        bidirected::{BidirectedChannel, IBidirectedChannel},
        MasterKey,
    };

    #[test]
    fn test() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel_pointer, mut data_pointer1, mut data_pointer2) =
            BidirectedChannel::create(0, 0, 10, 10);

        for i in 0..3 {
            let data_key = master_key.get_data_key();
            assert_eq!(*data_pointer1.get_input(&data_key), i);
            assert_eq!(*data_pointer2.get_input(&data_key), 10 - i);
            *data_pointer2.get_output(&data_key) = i + 1;
            *data_pointer1.get_output(&data_key) = 10 - (i + 1);

            let channel_key = data_key.into_channel_key();
            channel_pointer.flush(&channel_key);
        }

        let (read_only_data1, writable_data1, read_only_data2, writable_data2) =
            BidirectedChannel::destroy(channel_pointer, data_pointer1, data_pointer2);

        assert_eq!(read_only_data1, 3);
        assert_eq!(writable_data1, 3);
        assert_eq!(read_only_data2, 7);
        assert_eq!(writable_data2, 7);
    }

    #[test]
    fn ensure_channel_is_object_safe() {
        let mut master_key = unsafe { MasterKey::create_unlimited() };
        let (mut channel_pointer, mut data_pointer1, mut data_pointer2) =
            BidirectedChannel::create(1, 2, 3, 4);
        let dyn_channel_pointer: &mut dyn IBidirectedChannel = &mut channel_pointer;

        dyn_channel_pointer.flush(&master_key.get_channel_key());
        assert_eq!(*data_pointer1.get_input(&master_key.get_data_key()), 2);
        assert_eq!(*data_pointer1.get_output(&master_key.get_data_key()), 4);
        assert_eq!(*data_pointer2.get_input(&master_key.get_data_key()), 4);
        assert_eq!(*data_pointer2.get_output(&master_key.get_data_key()), 2);
        BidirectedChannel::destroy(channel_pointer, data_pointer1, data_pointer2);
    }
}
