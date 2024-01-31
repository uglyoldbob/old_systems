//! Common code for audio processing

/// An ringbuffer audio producer of several different kinds of data
pub enum AudioProducer {
    /// u8
    U8(
        ringbuf::Producer<
            u8,
            std::sync::Arc<ringbuf::SharedRb<u8, Vec<std::mem::MaybeUninit<u8>>>>,
        >,
    ),
    /// u16
    U16(
        ringbuf::Producer<
            u16,
            std::sync::Arc<ringbuf::SharedRb<u16, Vec<std::mem::MaybeUninit<u16>>>>,
        >,
    ),
    /// u32
    U32(
        ringbuf::Producer<
            u32,
            std::sync::Arc<ringbuf::SharedRb<u32, Vec<std::mem::MaybeUninit<u32>>>>,
        >,
    ),
    /// f32
    F32(
        ringbuf::Producer<
            f32,
            std::sync::Arc<ringbuf::SharedRb<f32, Vec<std::mem::MaybeUninit<f32>>>>,
        >,
    ),
}

impl AudioProducer {
    /// Push a slice of audiobuffer data into the producer, eventually making sound
    pub fn push_slice(&mut self, slice: &AudioBuffer) {
        match self {
            AudioProducer::U8(d) => match slice {
                AudioBuffer::U8(s) => {
                    d.push_slice(s);
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        let _ = d.push((t >> 8) as u8);
                    }
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        let _ = d.push((t >> 24) as u8);
                    }
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        let _ = d.push((t / 255.0) as u8);
                    }
                }
            },
            AudioProducer::U16(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        let _ = d.push((*t as u16) * 0x101);
                    }
                }
                AudioBuffer::U16(s) => {
                    d.push_slice(s);
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        let _ = d.push((*t >> 16) as u16);
                    }
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        let _ = d.push((t * 65535.0) as u16);
                    }
                }
            },
            AudioProducer::U32(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        let _ = d.push((*t as u32) * 0x1010101);
                    }
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        let _ = d.push((*t as u32) * 0x10001);
                    }
                }
                AudioBuffer::U32(s) => {
                    d.push_slice(s);
                }
                AudioBuffer::F32(s) => {
                    for t in s {
                        let _ = d.push((t * 4294967295.0) as u32);
                    }
                }
            },
            AudioProducer::F32(d) => match slice {
                AudioBuffer::U8(s) => {
                    for t in s {
                        let _ = d.push((*t as f32) / 255.0);
                    }
                }
                AudioBuffer::U16(s) => {
                    for t in s {
                        let _ = d.push((*t as f32) / 65535.0);
                    }
                }
                AudioBuffer::U32(s) => {
                    for t in s {
                        let _ = d.push((*t as f32) / 4294967295.0);
                    }
                }
                AudioBuffer::F32(s) => {
                    d.push_slice(s);
                }
            },
        }
    }

    /// Create an audio buffer with the same datatype as the producer, of the given size.
    pub fn make_buffer(&self, size: usize) -> AudioBuffer {
        println!("Make audio buffer size {}", size);
        match self {
            AudioProducer::U8(_) => AudioBuffer::U8(vec![0; size]),
            AudioProducer::U16(_) => AudioBuffer::U16(vec![0; size]),
            AudioProducer::U32(_) => AudioBuffer::U32(vec![0; size]),
            AudioProducer::F32(_) => AudioBuffer::F32(vec![0.0; size]),
        }
    }
}

/// An iterator over an audiobuffer
pub struct AudioBufferIterator<'a> {
    /// The audio buffer to iterate over.
    data: &'a AudioBuffer,
    /// The current index in iterating.
    index: usize,
}

impl<'a> Iterator for AudioBufferIterator<'a> {
    type Item = AudioSample;
    fn next(&mut self) -> Option<Self::Item> {
        let result = if self.index < self.data.len() {
            Some(match self.data {
                AudioBuffer::U8(d) => AudioSample::U8(d[self.index]),
                AudioBuffer::U16(d) => AudioSample::U16(d[self.index]),
                AudioBuffer::U32(d) => AudioSample::U32(d[self.index]),
                AudioBuffer::F32(d) => AudioSample::F32(d[self.index]),
            })
        } else {
            None
        };
        self.index += 1;
        result
    }
}

/// An audio buffer holding one of several datatypes.
pub enum AudioBuffer {
    /// u8
    U8(Vec<u8>),
    /// u16
    U16(Vec<u16>),
    /// u32
    U32(Vec<u32>),
    /// f32
    F32(Vec<f32>),
}

impl AudioBuffer {
    /// Return an iterator for the buffer.
    pub fn iter(&self) -> AudioBufferIterator {
        AudioBufferIterator {
            data: self,
            index: 0,
        }
    }

    /// Create a new buffer of f32 with the given size.
    pub fn new_f32(size: usize) -> Self {
        println!("Make audio bufferf size {}", size);
        Self::F32(vec![0.0; size])
    }

    /// Return a vec that can be used with gstreamer.
    pub fn gstreamer_slice(&self) -> Vec<u8> {
        match self {
            AudioBuffer::U8(d) => d.to_vec(),
            AudioBuffer::U16(d) => d.iter().flat_map(|a| a.to_le_bytes()).collect(),
            AudioBuffer::U32(d) => d.iter().flat_map(|a| a.to_le_bytes()).collect(),
            AudioBuffer::F32(d) => d.iter().flat_map(|a| a.to_bits().to_le_bytes()).collect(),
        }
    }

    /// Return the length of the buffer in number of elements.
    pub fn len(&self) -> usize {
        match self {
            AudioBuffer::U8(d) => d.len(),
            AudioBuffer::U16(d) => d.len(),
            AudioBuffer::U32(d) => d.len(),
            AudioBuffer::F32(d) => d.len(),
        }
    }

    /// Place the specified element in the specified index
    pub fn place(&mut self, index: usize, elem: AudioSample) {
        match self {
            AudioBuffer::U8(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s;
                }
                AudioSample::U16(s) => {
                    d[index] = (s >> 8) as u8;
                }
                AudioSample::U32(s) => {
                    d[index] = (s >> 24) as u8;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 255.0) as u8;
                }
            },
            AudioBuffer::U16(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s as u16 * 0x101;
                }
                AudioSample::U16(s) => {
                    d[index] = s;
                }
                AudioSample::U32(s) => {
                    d[index] = (s >> 16) as u16;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 65535.0) as u16;
                }
            },
            AudioBuffer::U32(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = s as u32 * 0x1010101;
                }
                AudioSample::U16(s) => {
                    d[index] = s as u32 * 0x10001;
                }
                AudioSample::U32(s) => {
                    d[index] = s;
                }
                AudioSample::F32(s) => {
                    d[index] = (s * 4294967295.0) as u32;
                }
            },
            AudioBuffer::F32(d) => match elem {
                AudioSample::U8(s) => {
                    d[index] = (s as f32) / 255.0;
                }
                AudioSample::U16(s) => {
                    d[index] = (s as f32) / 65535.0;
                }
                AudioSample::U32(s) => {
                    d[index] = (s as f32) / 4294967295.0;
                }
                AudioSample::F32(s) => {
                    d[index] = s;
                }
            },
        }
    }
}

#[derive(Copy, Clone)]
/// Represents a single sample of one channel of audio.
pub enum AudioSample {
    /// A u8 sample
    U8(u8),
    /// A u16 sample
    U16(u16),
    /// A u32 sample
    U32(u32),
    /// An f32 sample
    F32(f32),
}

/// The various ways of producing samples of data
enum AudioProducerMethod {
    /// A ring buffer is used to produce the audio
    RingBuffer(AudioProducer),
    /// The audio is pushed directly to gstreamer for recordings
    GStreamer(gstreamer_app::AppSrc),
}

impl AudioProducerMethod {
    /// Push a slice of data to the audio producer
    fn push_slice(&mut self, slice: &AudioBuffer) {
        match self {
            AudioProducerMethod::RingBuffer(rb) => {
                rb.push_slice(slice);
            }
            AudioProducerMethod::GStreamer(appsrc) => {
                let b: Vec<u8> = slice.gstreamer_slice();
                let buf = gstreamer::Buffer::from_slice(b);
                appsrc.do_timestamp();
                let _e = appsrc.push_buffer(buf).is_err();
            }
        }
    }
}

/// A struct that allows the producer and the rate of sample production to be linked together
pub struct AudioProducerWithRate {
    /// The number of clocks between generated samples
    interval: f32,
    /// The counter for generating samples at the right times
    counter: f32,
    /// The ringbuffer to put samples into
    producer: AudioProducerMethod,
    /// The buffer to put samples into
    buffer: AudioBuffer,
    /// The index of where to store samples
    buffer_index: usize,
}

impl AudioProducerWithRate {
    /// Build a new object
    pub fn new(producer: AudioProducer, size: usize) -> Self {
        let buf = producer.make_buffer(size);
        Self {
            interval: 1.0,
            counter: 0.0,
            producer: AudioProducerMethod::RingBuffer(producer),
            buffer: buf,
            buffer_index: 0,
        }
    }

    /// Create a new object and a new ringbuffer based on size
    pub fn new_gstreamer(size: usize, interval: f32, src: gstreamer_app::AppSrc) -> Self {
        Self {
            interval,
            counter: 0.0,
            producer: AudioProducerMethod::GStreamer(src),
            buffer: AudioBuffer::new_f32(size),
            buffer_index: 0,
        }
    }

    /// Set the interval for generating audio on this stream
    pub fn set_audio_interval(&mut self, i: f32) {
        self.interval = i;
    }

    /// Fill the local buffer from another audiobuffer
    pub fn fill_with_buffer(&mut self, buf: &AudioBuffer) {
        self.producer.push_slice(buf);
    }

    /// Create a buffer with the samer type specified by `t`, from the given slice of raw data, converting as necessary.
    pub fn make_buffer(&self, t: AudioSample, d: &[u8]) -> AudioBuffer {
        match t {
            AudioSample::U8(_) => AudioBuffer::U8(d.to_vec()),
            AudioSample::U16(_) => {
                let a: Vec<u16> = d
                    .chunks_exact(2)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .collect();
                AudioBuffer::U16(a)
            }
            AudioSample::U32(_) => {
                let a: Vec<u32> = d
                    .chunks_exact(4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .collect();
                AudioBuffer::U32(a)
            }
            AudioSample::F32(_) => {
                let a: Vec<f32> = d
                    .chunks_exact(4)
                    .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .collect();
                AudioBuffer::F32(a)
            }
        }
    }

    /// Fill the local audio buffer with data, returning a Some when it is full
    pub fn fill_audio_buffer(&mut self, sample: AudioSample) {
        self.counter += 1.0;
        if self.counter >= self.interval {
            self.counter -= self.interval;
            self.buffer.place(self.buffer_index, sample);
            let out = if self.buffer_index < (self.buffer.len() - 1) {
                self.buffer_index += 1;
                None
            } else {
                self.buffer_index = 0;
                Some(&self.buffer)
            };
            if let Some(out) = out {
                self.producer.push_slice(out);
            }
        }
    }
}
