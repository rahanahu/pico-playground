use bitfield::bitfield;
// core間fifoで使うメッセージの定義
// 32bitの先頭4bitを識別子として使用し、残りの28bitをデータとして使用する
// 0x0: SerialCMD

extern crate alloc;
use alloc::vec::Vec;
use defmt::info;

pub enum FifoMessageKind {
    SerialCMD(SerialCMD),
    PWMCMD(PWMCMD),
    VersionCMD(VersionCMD),
    Unknown(u32),
}

pub enum FifoMsgIdentifier {
    SerialCMD = 0x0,
    VersionCMD,
    PWMCMD,
}

enum SerialCommandType {
    PWM = 0x1,
    CFG,
    VERSION,
}

impl TryFrom<u8> for SerialCommandType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == SerialCommandType::PWM as u8 => Ok(SerialCommandType::PWM),
            x if x == SerialCommandType::CFG as u8 => Ok(SerialCommandType::CFG),
            x if x == SerialCommandType::VERSION as u8 => Ok(SerialCommandType::VERSION),
            _ => Err(()),
        }
    }
}
// シリアルメッセージの冒頭識別子の判定用
impl TryFrom<&str> for SerialCommandType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            s if s.starts_with("PWM") => Ok(SerialCommandType::PWM),
            s if s.starts_with("CFG") => Ok(SerialCommandType::CFG),
            s if s.starts_with("VER") => Ok(SerialCommandType::VERSION),
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
            x if x == FifoMsgIdentifier::SerialCMD as u8 => Ok(FifoMsgIdentifier::SerialCMD),
            x if x == FifoMsgIdentifier::VersionCMD as u8 => Ok(FifoMsgIdentifier::VersionCMD),
            x if x == FifoMsgIdentifier::PWMCMD as u8 => Ok(FifoMsgIdentifier::PWMCMD),
            _ => Err(()),
        }
    }
}

impl From<FifoMsgFrame> for FifoMessageKind {
    fn from(f: FifoMsgFrame) -> Self {
        match FifoMsgIdentifier::try_from(f.identifier()) {
            Ok(FifoMsgIdentifier::SerialCMD) => FifoMessageKind::SerialCMD(SerialCMD(f.payload())),
            Ok(FifoMsgIdentifier::PWMCMD) => FifoMessageKind::PWMCMD(PWMCMD(f.payload())),
            Ok(FifoMsgIdentifier::VersionCMD) => {
                FifoMessageKind::VersionCMD(VersionCMD(f.payload()))
            }
            Err(_) => FifoMessageKind::Unknown(f.payload()),
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
    pub u8, cmd, set_cmd: 27, 24;
    pub u8, channel, set_channel: 23, 18;
    pub u32, value, set_value: 17, 0;
}

bitfield! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct PWMCMD(u32);
    impl Debug;
    pub u8, channel, set_channel: 27, 22;
    pub u32, value, set_value: 23, 0;
}
bitfield! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct VersionCMD(u32);
    impl Debug;
    pub u8, major, set_major: 27, 20;
    pub u8, minor, set_minor: 19, 12;
    pub u8, patch, set_patch: 11, 4;
}

pub fn encode_cmd(s: &str) -> Option<Vec<FifoMsgFrame>> {
    let cmd_type = SerialCommandType::try_from(s).ok()?;
    info!("Received command: {}", s);
    match cmd_type {
        SerialCommandType::PWM => {
            info!("Received PWM command!!: {}", s);
            let cmd_body = s.strip_prefix("PWM")?;
            let cmds = cmd_body.split(',').collect::<Vec<&str>>();
            if cmds.len() < 1 {
                info!("Invalid PWM command format: {}", s);
                return None; // Invalid command format
            } else {
                let mut frames = Vec::new();
                for cmd in cmds {
                    let parts: Vec<&str> = cmd.split(':').collect();
                    if parts.len() != 2 {
                        info!("Invalid PWM command part format2: {}", cmd);
                        return None; // Invalid command format
                    }
                    let channel: u8 = parts[0].parse().ok()?;
                    let value: u32 = parts[1].parse().ok()?;
                    // パッキング
                    let mut pwmcmd = PWMCMD(0);
                    pwmcmd.set_channel(channel);
                    pwmcmd.set_value(value);
                    let mut frame = FifoMsgFrame(0);
                    frame.set_identifier(FifoMsgIdentifier::PWMCMD as u8);
                    frame.set_payload(pwmcmd.0);
                    frames.push(frame);
                }
                Some(frames)
            }
        }
        SerialCommandType::VERSION => {
            info!("Received VERSION command!!: {}", s);
            let mut versions = Vec::new();
            let mut version_cmd = VersionCMD(0);

            let version = env!("CARGO_PKG_VERSION");
            version.split('.').enumerate().for_each(|(i, v)| {
                let num: u8 = v.parse().unwrap_or(0);
                match i {
                    0 => version_cmd.set_major(num),
                    1 => version_cmd.set_minor(num),
                    2 => version_cmd.set_patch(num),
                    _ => {}
                }
            });

            let mut frame = FifoMsgFrame(0);
            frame.set_identifier(FifoMsgIdentifier::VersionCMD as u8);
            frame.set_payload(version_cmd.0);
            versions.push(frame);
            Some(versions)
        }
        SerialCommandType::CFG => {
            info!("Received CFG command!!");
            // Handle other command types if needed
            None
        }
    }
}

pub fn decode_fifo_msg(frame: FifoMsgFrame) -> FifoMessageKind {
    frame.into()
}
