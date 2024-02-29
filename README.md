# Two-Phase Channel

Various safe synchronisation-free parallel communication channels.
The channels support the transmission of one data item, i.e. they have no queue.
The synchronisation is externalised by requiring zero-sized key types to access the channels.
This is useful if a computation can be split into separate *compute* and *communicate* steps.
Then, the two-phase channel allows the steps to work without any internal synchronisation, the threads only need to be synchronised between the steps.

While this libary was made with performance in mind, it is unclear if this pattern actually improves performance for any given computational task.
Use at your own discretion.
