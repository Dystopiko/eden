use thiserror::Error;
use twilight_model::id::Id;

use std::fmt;
use std::ops;

/// Database compatible type for any IDs in [`twilight_model`].
///
/// This type automatically dereferences to its true type so it can
/// be treated like those implemented from [twilight].
///
/// [twilight]: twilight_model
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Snowflake(Id<()>);

impl Snowflake {
    #[must_use]
    pub const fn new(id: Id<()>) -> Self {
        Self(id.cast())
    }
}

impl<T> From<Id<T>> for Snowflake {
    fn from(value: Id<T>) -> Self {
        Self(value.cast())
    }
}

impl<'a, T> From<&'a Id<T>> for Snowflake {
    fn from(value: &'a Id<T>) -> Self {
        Self(value.cast())
    }
}

impl<'a, T> From<&'a mut Id<T>> for Snowflake {
    fn from(value: &'a mut Id<T>) -> Self {
        Self(value.cast())
    }
}

impl<T> From<Snowflake> for Id<T> {
    fn from(val: Snowflake) -> Self {
        val.0.cast()
    }
}

impl ops::Deref for Snowflake {
    type Target = Id<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Snowflake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Error)]
#[error("got invalid value for a Discord snowflake")]
struct InvalidValueError;

impl<'row> sqlx::Decode<'row, sqlx::Sqlite> for Snowflake
where
    i64: sqlx::Decode<'row, sqlx::Sqlite>,
{
    // Discord uses up to 63 bits in 64-bit signed integer for
    // their snowflake ID anyways. No sign loss or unexpected
    // output will happen. :)
    //
    // Reference: https://discord.com/developers/docs/reference#snowflakes-snowflake-id-format-structure-left-to-right
    #[allow(clippy::cast_sign_loss)]
    fn decode(value: sqlx::sqlite::SqliteValueRef<'row>) -> Result<Self, sqlx::error::BoxDynError> {
        // Make sure the value is not negative nor zero, this is very important
        // for 64 bit unsigned non-zero integers.
        let value = i64::decode(value)?;
        if value.is_negative() {
            return Err(Box::new(InvalidValueError));
        }

        if let Some(id) = Id::new_checked(value as u64) {
            Ok(Self(id))
        } else {
            Err(Box::new(InvalidValueError))
        }
    }
}

#[derive(Debug, Error)]
#[error("got invalid snowflake ID for {0:?} (out of bounds to i64)")]
struct OutOfBoundsError(u64);

impl<'query> sqlx::Encode<'query, sqlx::Sqlite> for Snowflake
where
    i64: sqlx::Encode<'query, sqlx::Sqlite>,
{
    // Twilight does not validate if there's an exceeding bit/s beyond 63 bits of snowflake
    // data as referenced to the Discord's snowflake ID structure, we need to check if we
    // have ONLY 63 BITS inside this type.
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'query>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let value = self.0.get();
        if let Ok(value) = i64::try_from(value) {
            value.encode(buf)
        } else {
            Err(Box::new(OutOfBoundsError(value)))
        }
    }
}

impl sqlx::Type<sqlx::Sqlite> for Snowflake {
    fn compatible(ty: &<sqlx::Sqlite as sqlx::Database>::TypeInfo) -> bool {
        <i64 as sqlx::Type<sqlx::Sqlite>>::compatible(ty)
    }

    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <i64 as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_ok;
    use eden_sqlite::Pool;
    use sqlx::Row;
    use twilight_model::id::Id;

    use crate::snowflake::Snowflake;

    #[tokio::test]
    async fn test_encode() {
        eden_common::testing::init();

        let pool = Pool::memory(None).await;
        let result = sqlx::query("SELECT ?")
            .bind(Snowflake::new(Id::new(123)))
            .execute(&mut *pool.acquire().await.unwrap())
            .await;

        assert_ok!(&result);
        insta::assert_debug_snapshot!(result.unwrap());
    }

    #[tokio::test]
    async fn test_encode_error() {
        eden_common::testing::init();

        // numbers beyond positive i64 limit are invalid
        let pool = Pool::memory(None).await;
        let error = sqlx::query("SELECT ?")
            .bind(Snowflake::new(Id::new((i64::MAX as u64) + 1)))
            .execute(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap_err();

        insta::assert_debug_snapshot!(error);
    }

    #[tokio::test]
    async fn test_decoding_negative_numbers() {
        eden_common::testing::init();

        // numbers beyond positive i64 limit are invalid
        let pool = Pool::memory(None).await;
        let row = sqlx::query("SELECT -1")
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        let error = row.try_get::<Snowflake, _>(0).unwrap_err();
        insta::assert_debug_snapshot!(error);
    }

    #[tokio::test]
    async fn test_decoding_zero() {
        eden_common::testing::init();

        // numbers beyond positive i64 limit are invalid
        let pool = Pool::memory(None).await;
        let row = sqlx::query("SELECT 0")
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        let error = row.try_get::<Snowflake, _>(0).unwrap_err();
        insta::assert_debug_snapshot!(error);
    }

    #[tokio::test]
    async fn test_decoding() {
        eden_common::testing::init();

        // numbers beyond positive i64 limit are invalid
        let pool = Pool::memory(None).await;
        let row = sqlx::query("SELECT 1")
            .fetch_one(&mut *pool.acquire().await.unwrap())
            .await
            .unwrap();

        let snowflake = row.try_get::<Snowflake, _>(0).unwrap();
        assert_eq!(snowflake, Snowflake::new(Id::new(1)));
    }
}
