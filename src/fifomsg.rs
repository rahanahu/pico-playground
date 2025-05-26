use bitfield::bitfield;
// core間fifoで使うメッセージの定義
// 32bitの先頭4bitを識別子として使用し、残りの28bitをデータとして使用する
// 0x0: SerialCMD

pub enum FifoMessage {
    SerialCMD(SerialCMD),
    Unknown(u32),
}

pub enum FifoMsgIdentifier {
    SerialCMD = 0x0,
}

pub enum SerialCommandType {
    PWM = 0x0,
    Analog,
}

impl TryFrom<u8> for SerialCommandType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(SerialCommandType::PWM),
            0x1 => Ok(SerialCommandType::Analog),
            _ => Err(()),
        }
    }
}

impl From<SerialCommandType> for u8 {
    fn from(cmd: SerialCommandType) -> Self {
        cmd as u8
    }
}

impl TryFrom<u8> for FifoMsgIdentifier {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x0 => Ok(FifoMsgIdentifier::SerialCMD),
            _ => Err(()),
        }
    }
}

impl From<FifoMsgFrame> for FifoMessage {
    fn from(f: FifoMsgFrame) -> Self {
        match FifoMsgIdentifier::try_from(f.identifier()) {
            Ok(FifoMsgIdentifier::SerialCMD) => FifoMessage::SerialCMD(SerialCMD(f.payload())),
            _ => FifoMessage::Unknown(f.payload()),
        }
    }
}

bitfield! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct FifoMsgFrame(u32);
    impl Debug;
    u8, identifier,set_identifier: 31,28;
    u32, payload, set_payload: 27,0;
}

bitfield! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct SerialCMD(u32);
    impl Debug;
    u8, cmd, set_cmd: 27, 24;
    u32, value, set_value: 23, 0;
}
