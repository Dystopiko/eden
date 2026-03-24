use std::{net::IpAddr, str::FromStr};

pub struct IpAddrString(String);

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for IpAddrString
where
    String: sqlx::Decode<'r, sqlx::Sqlite>,
{
    fn decode(
        value: <sqlx::Sqlite as sqlx::Database>::ValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        String::decode(value).map(Self)
    }
}

impl sqlx::Type<sqlx::Sqlite> for IpAddrString {
    fn type_info() -> <sqlx::Sqlite as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }

    fn compatible(ty: &<sqlx::Sqlite as sqlx::Database>::TypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Sqlite>>::compatible(ty)
    }
}

impl TryFrom<IpAddrString> for IpAddr {
    type Error = <IpAddr as FromStr>::Err;

    fn try_from(value: IpAddrString) -> Result<Self, Self::Error> {
        value.0.parse()
    }
}
