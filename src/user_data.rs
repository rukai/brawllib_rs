#[derive(Clone, Debug)]
pub struct UserData {
    pub name: String,
    pub value: UserDataValue,
}

#[derive(Clone, Debug)]
pub enum UserDataValue {
    Int(i32),
    Float(f32),
    String(String),
}
