use crate::errors::ApiError;

#[test]
fn test_serialization() {
    let error = ApiError::READONLY_MODE;
    let json = serde_json::to_string_pretty(&error).unwrap();
    insta::assert_snapshot!(json);
}
