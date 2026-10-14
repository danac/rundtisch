/// UUIDv4 from `getrandom`, same construction as auth `public_id`.
pub(crate) fn new_uuid_v4() -> uuid::Uuid {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("CSPRNG available for UUIDv4");
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_uuid_is_v4_and_unique() {
        let id = new_uuid_v4();
        assert_eq!(id.get_version(), Some(uuid::Version::Random));
        assert_ne!(id, new_uuid_v4());
    }
}
