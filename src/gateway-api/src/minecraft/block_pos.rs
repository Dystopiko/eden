use serde::{Deserialize, Serialize, ser::SerializeSeq};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlockPos {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl BlockPos {
    pub const ZERO: BlockPos = BlockPos::new(0, 0, 0);

    #[must_use]
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }
}

impl Default for BlockPos {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<'de> Deserialize<'de> for BlockPos {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(BlockPosVisitor)
    }
}

struct BlockPosVisitor;

impl<'de> serde::de::Visitor<'de> for BlockPosVisitor {
    type Value = BlockPos;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Minecraft block position")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let x: i64 = seq.next_element()?.ok_or_else(|| {
            serde::de::Error::custom("Minecraft block position must be in three elements")
        })?;

        let y: i64 = seq.next_element()?.ok_or_else(|| {
            serde::de::Error::custom("Minecraft block position must be in three elements")
        })?;

        let z: i64 = seq.next_element()?.ok_or_else(|| {
            serde::de::Error::custom("Minecraft block position must be in three elements")
        })?;

        if seq.next_element::<i64>()?.is_some() {
            return Err(serde::de::Error::custom(
                "Minecraft block position must be in three elements",
            ));
        }

        Ok(BlockPos { x, y, z })
    }
}

impl Serialize for BlockPos {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(3))?;
        serializer.serialize_element(&self.x)?;
        serializer.serialize_element(&self.y)?;
        serializer.serialize_element(&self.z)?;
        serializer.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::minecraft::BlockPos;

    #[test]
    fn test_serialization() {
        let pos = BlockPos::ZERO;
        insta::assert_json_snapshot!(&pos);
    }
}
