//! Code for storage structures

use std::path::PathBuf;

/// A visitor for PersistentStorage
pub struct PersistentVisitor;

impl<'de> serde::de::Visitor<'de> for PersistentVisitor {
    type Value = PersistentStorage;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("byte sequence")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let len = std::cmp::min(visitor.size_hint().unwrap_or(0), 4096);
        let mut bytes = Vec::with_capacity(len);

        let t: Option<u8> = visitor.next_element()?;
        while let Some(b) = visitor.next_element()? {
            bytes.push(b);
        }
        if let Some(t) = t {
            match t {
                0 | 1 => Ok(PersistentStorage::new_should(bytes)),
                _ => Ok(PersistentStorage::new_volatile(bytes)),
            }
        } else {
            Ok(PersistentStorage::new_volatile(bytes))
        }
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let v = v.as_bytes().to_vec();
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let v = v.into_bytes();
        match v[0] {
            0 | 1 => Ok(PersistentStorage::new_should(v[1..].to_vec())),
            _ => Ok(PersistentStorage::new_volatile(v[1..].to_vec())),
        }
    }
}

/// A vec that could be battery backed
pub enum PersistentStorage {
    /// The vec is battery backed by a file
    Persistent(PathBuf, memmap2::MmapMut),
    /// The vec should be persistent but it is not for some reason
    ShouldBePersistent(Vec<u8>),
    /// The vec is simply a plain vector
    Volatile(Vec<u8>),
}

impl Clone for PersistentStorage {
    fn clone(&self) -> Self {
        match self {
            Self::Persistent(_pb, arg0) => Self::ShouldBePersistent(arg0.to_vec()),
            Self::ShouldBePersistent(v) => Self::ShouldBePersistent(v.clone()),
            Self::Volatile(arg0) => Self::Volatile(arg0.clone()),
        }
    }
}

impl serde::Serialize for PersistentStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let elem = self.contents();
        let mut seq = serializer.serialize_seq(Some(elem.len() + 1))?;
        match self {
            PersistentStorage::Volatile(_v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &2_u8)?;
            }
            PersistentStorage::Persistent(_pb, _v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &0_u8)?;
            }
            PersistentStorage::ShouldBePersistent(_v) => {
                serde::ser::SerializeSeq::serialize_element(&mut seq, &1_u8)?;
            }
        }
        for e in elem {
            serde::ser::SerializeSeq::serialize_element(&mut seq, e)?;
        }
        serde::ser::SerializeSeq::end(seq)
    }
}

impl<'de> serde::Deserialize<'de> for PersistentStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(PersistentVisitor)
    }
}

impl std::ops::Index<usize> for PersistentStorage {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.contents()[index]
    }
}

impl std::ops::IndexMut<usize> for PersistentStorage {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.contents_mut()[index]
    }
}

impl Drop for PersistentStorage {
    fn drop(&mut self) {
        if let PersistentStorage::Persistent(_p, v) = self {
            let _ = v.flush();
        }
    }
}

impl PersistentStorage {
    /// Create a persistent storage object using the specified path and data. Overwrite will overwrite the contents of the file if set to true.
    fn make_persistent(p: PathBuf, v: Vec<u8>, overwrite: bool) -> Option<Self> {
        let file = if p.exists() {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&p);
            if let Ok(mut file) = file {
                if overwrite {
                    std::io::Write::write_all(&mut file, &v[..]).ok()?;
                }
                Some(file)
            } else {
                None
            }
        } else {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&p);
            if let Ok(mut file) = file {
                std::io::Write::write_all(&mut file, &v[..]).ok()?;
                Some(file)
            } else {
                None
            }
        };
        if let Some(file) = file {
            let mm = unsafe { memmap2::MmapMut::map_mut(&file) };
            if let Ok(mm) = mm {
                Some(PersistentStorage::Persistent(p, mm))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Reupgrade the object to be fully persistent
    pub fn upgrade_to_persistent(&mut self, p: PathBuf) {
        if let PersistentStorage::ShouldBePersistent(v) = self {
            if let Some(ps) = Self::make_persistent(p, v.clone(), true) {
                *self = ps;
            }
        }
    }

    /// Convert this volatile object to a non-volatile object, taking on the contents of the existing nonvolatile storage if it exists.
    /// If it does not exist, then the current contents are transferred over.
    pub fn convert_to_nonvolatile(&mut self, p: PathBuf) {
        let t = match self {
            PersistentStorage::Persistent(_pb, _a) => None,
            PersistentStorage::ShouldBePersistent(_v) => None,
            PersistentStorage::Volatile(v) => Self::make_persistent(p, v.clone(), false),
        };
        if let Some(t) = t {
            *self = t;
        }
    }

    /// Create a new object that should be persistent
    fn new_should(v: Vec<u8>) -> Self {
        PersistentStorage::ShouldBePersistent(v)
    }

    /// Create a new volatile storage object
    fn new_volatile(v: Vec<u8>) -> Self {
        PersistentStorage::Volatile(v)
    }

    /// Convenience function for determining if the contents are empty.
    pub fn is_empty(&self) -> bool {
        self.contents().is_empty()
    }

    /// The length of the contents
    pub fn len(&self) -> usize {
        self.contents().len()
    }

    /// Get chunks of the data
    pub fn chunks_exact(&self, cs: usize) -> std::slice::ChunksExact<'_, u8> {
        self.contents().chunks_exact(cs)
    }

    /// Retrieve a reference to the contents
    fn contents(&self) -> &[u8] {
        match self {
            PersistentStorage::Persistent(_pb, mm) => mm.as_ref(),
            PersistentStorage::ShouldBePersistent(v) => &v[..],
            PersistentStorage::Volatile(v) => &v[..],
        }
    }

    /// Retrieve a mutable reference to the contents
    fn contents_mut(&mut self) -> &mut [u8] {
        match self {
            PersistentStorage::Persistent(_pb, mm) => mm.as_mut(),
            PersistentStorage::ShouldBePersistent(v) => &mut v[..],
            PersistentStorage::Volatile(v) => &mut v[..],
        }
    }
}