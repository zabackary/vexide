//! Generic serial device module.
//!
//! Provides support for using [`SmartPort`]s as generic serial communication devices.

use alloc::ffi::CString;

use embedded_io::{ErrorType, Read, Write};
use snafu::Snafu;
use vex_sdk::{
    vexDeviceGenericRadioConnection, vexDeviceGenericRadioLinkStatus, vexDeviceGenericRadioReceive,
    vexDeviceGenericRadioReceiveAvail, vexDeviceGenericRadioTransmit,
    vexDeviceGenericRadioWriteFree, V5_DeviceT,
};

use super::{SmartDevice, SmartDeviceType, SmartPort};
use crate::PortError;

/// Represents a smart port configured as a VEXLink radio.
///
/// VEXLink is a point-to-point wireless communications protocol between
/// two VEXNet radios. For further information, see <https://www.vexforum.com/t/vexlink-documentaton/84538>
#[derive(Debug, Eq, PartialEq)]
pub struct RadioLink {
    port: SmartPort,
    device: V5_DeviceT,
}

// SAFETY: Required because we store a raw pointer to the device handle to avoid it getting from the
// SDK each device function. Simply sharing a raw pointer across threads is not inherently unsafe.
unsafe impl Send for RadioLink {}
unsafe impl Sync for RadioLink {}

impl RadioLink {
    /// Opens a radio link from a VEXNet radio plugged into a smart port. Once
    /// opened, other VEXNet functionality such as controller tethering on this
    /// radio will be disabled.
    ///
    /// # Examples
    ///
    /// ```
    /// let link = RadioLink::open(port_1, "643A", LinkType::Manager)?;
    /// ```
    pub fn open(port: SmartPort, id: &str, link_type: LinkType) -> Result<Self, LinkError> {
        // Ensure that a radio is plugged into the requested port.
        //
        // Once we call [`vexDeviceGenericRadioConnection`], this type
        // will be changed to be generic serial, but we haven't called
        // it yet.
        port.validate_type(SmartDeviceType::Radio)?;

        // That this constructor literally has to be fallible unlike others.
        unsafe {
            vexDeviceGenericRadioConnection(
                port.device_handle(),
                CString::new(id)
                    .map_err(|_| LinkError::NonTerminatingNul)?
                    .into_raw(),
                match link_type {
                    LinkType::Worker => 0,
                    LinkType::Manager => 1,
                },
                true,
            );
        }

        Ok(Self {
            device: unsafe { port.device_handle() },
            port,
        })
    }

    /// Returns the number of bytes available to be read in the the radio's input buffer.
    pub fn unread_bytes(&self) -> Result<usize, LinkError> {
        self.validate_port()?;

        Ok(unsafe { vexDeviceGenericRadioReceiveAvail(self.device) } as usize)
    }

    /// Returns the number of bytes free in the radio's output buffer.
    pub fn available_write_bytes(&self) -> Result<usize, LinkError> {
        self.validate_port()?;

        match unsafe { vexDeviceGenericRadioWriteFree(self.device) } {
            // TODO: This check may not be necessary, since PROS doesn't do it,
            //		 but we do it just to be safe.
            -1 => Err(LinkError::ReadFailed),
            available => Ok(available as usize),
        }
    }

    /// Returns `true` if there is a link established with another radio.
    pub fn is_linked(&self) -> Result<bool, LinkError> {
        self.validate_port()?;

        Ok(unsafe { vexDeviceGenericRadioLinkStatus(self.device) })
    }
}

#[cfg(feature = "std")]
impl std::io::Read for RadioLink {
    /// Read some bytes sent to the radio into the specified buffer, returning
    /// how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let is_linked = self.is_linked().map_err(|e| match e {
            LinkError::Port { source } => match source {
                PortError::Disconnected => {
                    std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Port does not exist.")
                }
                PortError::IncorrectDevice => std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "Port is in use as another device.",
                ),
            },
            _ => unreachable!(),
        })?;

        if !is_linked {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Radio is not linked!",
            ));
        }

        match unsafe {
            vexDeviceGenericRadioReceive(self.device, buf.as_mut_ptr(), buf.len() as u16)
        } {
            -1 => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Internal read error occurred.",
            )),
            recieved => Ok(recieved as usize),
        }
    }
}

#[cfg(feature = "std")]
impl std::io::Write for RadioLink {
    /// Write a buffer into the radio's output buffer, returning how many bytes
    /// were written.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let is_linked = self.is_linked().map_err(|e| match e {
            LinkError::Port { source } => match source {
                PortError::Disconnected => {
                    std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Port does not exist.")
                }
                PortError::IncorrectDevice => std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "Port is in use as another device.",
                ),
            },
            _ => unreachable!(),
        })?;

        if !is_linked {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Radio is not linked!",
            ));
        }

        match unsafe { vexDeviceGenericRadioTransmit(self.device, buf.as_ptr(), buf.len() as u16) }
        {
            -1 => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Internal write error occurred.",
            )),
            written => Ok(written as usize),
        }
    }

    /// This function does nothing.
    ///
    /// VEXLink immediately sends and clears data sent into the write buffer.
    fn flush(&mut self) -> std::io::Result<()> {
        if !self.is_linked().map_err(|e| match e {
            LinkError::Port { source } => match source {
                PortError::Disconnected => {
                    std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Port does not exist.")
                }
                PortError::IncorrectDevice => std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    "Port is in use as another device.",
                ),
            },
            _ => unreachable!(),
        })? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Radio is not linked!",
            ));
        }

        Ok(())
    }
}

impl SmartDevice for RadioLink {
    fn port_number(&self) -> u8 {
        self.port.number()
    }

    fn device_type(&self) -> SmartDeviceType {
        SmartDeviceType::GenericSerial
    }
}

impl ErrorType for RadioLink {
    type Error = LinkError;
}

/// The type of a radio link being established.
///
/// VEXLink is a point-to-point connection, with one "manager" robot and
/// one "worker" robot.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LinkType {
    /// Manager Radio
    ///
    /// This end of the link has a 1040 bytes/sec data rate when
    /// communicating with a worker radio.
    Manager,

    /// Worker Radio
    ///
    /// This end of the link has a 520 bytes/sec data rate when
    /// communicating with a manager radio.
    Worker,
}

/// Errors that can occur when interacting with a [`RadioLink`].
#[derive(Debug, Snafu)]
pub enum LinkError {
    /// Not linked with another radio.
    NotLinked,

    /// Internal write error occurred.
    WriteFailed,

    /// Internal read error occurred.
    ReadFailed,

    /// CString::new encountered NUL (U+0000) byte in non-terminating position.
    NonTerminatingNul,

    /// Generic port related error.
    #[snafu(display("{source}"), context(false))]
    Port {
        /// The source of the error.
        source: PortError,
    },
}


impl embedded_io::Error for LinkError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}
