use std::mem;

pub struct MoosMessage {
    pub length: i32,
    pub id: i32,
    pub message_type: i8,
    pub data_type: i8,
    pub double_value: f64,
    pub double_value2: f64,
    pub string_value: String,
    pub key: String,
    pub time: f64,
    pub source: String,
    pub source_aux: String,
    pub originating_community: String,
}

/*
 * length
 * id
 * message_type
 * data_type
 * source
 * source_aux
 * originating_community
 * key
 * time
 * double_value
 * double_value2
 * string_value - // @TODO how to handle binary data
 *
 */
impl MoosMessage {
    pub fn get_size(&self) -> i32 {
        // (mem::size_of(self.id) +
        //mem::size_of(self.message_type)) as i32

        (mem::size_of_val(&self.id) + mem::size_of_val(&self.double_value)) as i32
    }
}

pub fn serialize(message: MoosMessage) {
    let mut data = Vec::new();
    data.push(message.get_size().to_le());
}
