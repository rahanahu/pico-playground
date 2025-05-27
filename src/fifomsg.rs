use bitfield::bitfield;
// core間fifoで使うメッセージの定義
// 32bitの先頭4bitを識別子として使用し、残りの28bitをデータとして使用する
// 0x0: SerialCMD

extern crate alloc;
use alloc::vec::Vec;
use defmt::info;

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

bitfield! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct LedCMD(u32);
    impl Debug;
    pub u32, cmd, set_cmd: 27, 0;
}

#[repr(u8)]
pub enum FifoMsgIdentifier {
    SerialCMD = 0x0,
    VersionCMD,
    PWMCMD,
    LedCMD,
}

pub enum FifoMessageKind {
    SerialCMD(SerialCMD),
    PWMCMD(PWMCMD),
    VersionCMD(VersionCMD),
    LedCMD(LedCMD),
    Unknown(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SerialCommandType {
    Pwm,
    Cfg,
    Version,
    Led,
}

impl SerialCommandType {
    const VARIANTS: &'static [(&'static str, SerialCommandType)] = &[
        ("PWM", SerialCommandType::Pwm),
        ("CFG", SerialCommandType::Cfg),
        ("VER", SerialCommandType::Version),
        ("LED", SerialCommandType::Led),
    ];
}

// FIFOメッセージ定義トレイト
pub trait FifoMessageDef: Sized {
    const IDENT: FifoMsgIdentifier;
    fn encode(&self) -> FifoMsgFrame {
        let mut frame = FifoMsgFrame(0);
        frame.set_identifier(Self::IDENT as u8);
        frame.set_payload(self.as_u32());
        frame
    }
    fn decode(payload: u32) -> Self;
    fn as_u32(&self) -> u32;
}

macro_rules! impl_fifo_msg {
    ($name:ident, $ident:expr) => {
        impl FifoMessageDef for $name {
            const IDENT: FifoMsgIdentifier = $ident;
            fn decode(payload: u32) -> Self {
                Self(payload)
            }
            fn as_u32(&self) -> u32 {
                self.0
            }
        }
    };
}

impl TryFrom<u8> for SerialCommandType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::VARIANTS
            .iter()
            .enumerate()
            .find_map(|(idx, &(_, variant))| {
                if idx as u8 == value {
                    Some(variant)
                } else {
                    None
                }
            })
            .ok_or(())
    }
}

impl From<SerialCommandType> for u8 {
    fn from(cmd: SerialCommandType) -> Self {
        SerialCommandType::VARIANTS
            .iter()
            .position(|&(_, v)| v == cmd)
            .unwrap_or(0) as u8
    }
}

impl TryFrom<&str> for SerialCommandType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::VARIANTS
            .iter()
            .find_map(|&(prefix, variant)| {
                if value.starts_with(prefix) {
                    Some(variant)
                } else {
                    None
                }
            })
            .ok_or(())
    }
}

impl TryFrom<u8> for FifoMsgIdentifier {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == FifoMsgIdentifier::SerialCMD as u8 => Ok(FifoMsgIdentifier::SerialCMD),
            x if x == FifoMsgIdentifier::VersionCMD as u8 => Ok(FifoMsgIdentifier::VersionCMD),
            x if x == FifoMsgIdentifier::PWMCMD as u8 => Ok(FifoMsgIdentifier::PWMCMD),
            x if x == FifoMsgIdentifier::LedCMD as u8 => Ok(FifoMsgIdentifier::LedCMD),
            _ => Err(()),
        }
    }
}

impl From<FifoMsgFrame> for FifoMessageKind {
    fn from(f: FifoMsgFrame) -> Self {
        match FifoMsgIdentifier::try_from(f.identifier()) {
            Ok(id) => decode_by_id(id, f.payload()),
            Err(_) => FifoMessageKind::Unknown(f.payload()),
        }
    }
}

fn decode_by_id(id: FifoMsgIdentifier, payload: u32) -> FifoMessageKind {
    match id {
        FifoMsgIdentifier::SerialCMD => FifoMessageKind::SerialCMD(SerialCMD::decode(payload)),
        FifoMsgIdentifier::PWMCMD => FifoMessageKind::PWMCMD(PWMCMD::decode(payload)),
        FifoMsgIdentifier::VersionCMD => FifoMessageKind::VersionCMD(VersionCMD::decode(payload)),
        FifoMsgIdentifier::LedCMD => FifoMessageKind::LedCMD(LedCMD::decode(payload)),
    }
}

pub fn encode_cmd(s: &str) -> Option<Vec<FifoMsgFrame>> {
    let cmd_type = SerialCommandType::try_from(s).ok()?;
    info!("Received command: {}", s);
    match cmd_type {
        SerialCommandType::Pwm => {
            // info!("Received PWM command!!: {}", s);
            let cmd_body = s.strip_prefix("PWM")?;
            let cmds = cmd_body.split(',').collect::<Vec<&str>>();
            if cmds.is_empty() {
                // info!("Invalid PWM command format: {}", s);
                None // Invalid command format
            } else {
                let mut frames = Vec::new();
                for cmd in cmds {
                    let parts: Vec<&str> = cmd.split(':').collect();
                    if parts.len() != 2 {
                        // info!("Invalid PWM command part format2: {}", cmd);
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
        SerialCommandType::Version => {
            // info!("Received VERSION command!!: {}", s);
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
            // versionは1つしかないのでVecでなくても良いが、空気を読んでvecにしておく
            Some(versions)
        }
        SerialCommandType::Cfg => {
            info!("Received CFG command!!");
            // Handle other command types if needed
            None
        }
        SerialCommandType::Led => {
            let cmd_body = s.strip_prefix("LED:")?;
            let mut leds = Vec::new();
            let mut led = LedCMD(0);
            match cmd_body {
                "ON" => led.set_cmd(1),
                "OFF" => led.set_cmd(0),
                "TOGGLE" => led.set_cmd(2),
                _ => {}
            }
            let mut frame = FifoMsgFrame(0);
            frame.set_identifier(FifoMsgIdentifier::LedCMD as u8);
            frame.set_payload(led.0);
            leds.push(frame);
            Some(leds)
        }
    }
}

pub fn decode_fifo_msg(frame: FifoMsgFrame) -> FifoMessageKind {
    frame.into()
}

impl_fifo_msg!(SerialCMD, FifoMsgIdentifier::SerialCMD);
impl_fifo_msg!(PWMCMD, FifoMsgIdentifier::PWMCMD);
impl_fifo_msg!(VersionCMD, FifoMsgIdentifier::VersionCMD);
impl_fifo_msg!(LedCMD, FifoMsgIdentifier::LedCMD);
