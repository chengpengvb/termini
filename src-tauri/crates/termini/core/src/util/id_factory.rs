use uuid::Uuid;

pub fn uuid() -> String {
    let full_uuid = Uuid::new_v4().simple().to_string();
    full_uuid[..21].to_string()
}
