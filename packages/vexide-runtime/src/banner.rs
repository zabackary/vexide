//! vexide startup banner

use std::time::Duration;

use vex_sdk::{
    vexBatteryCapacityGet, vexCompetitionStatus, vexSystemPowerupTimeGet, vexSystemVersion,
};

#[allow(unused)]
macro_rules! ansi_rgb {
    ($r:expr, $g:expr, $b:expr) => {
        concat!("\x1B[38;2;", $r, ";", $g, ";", $b, "m")
    };
}

macro_rules! ansi_rgb_bold {
    ($r:expr, $g:expr, $b:expr) => {
        concat!("\x1B[1;38;2;", $r, ";", $g, ";", $b, "m")
    };
}

#[derive(Clone, Copy, Debug)]
struct BannerTheme {
    pub emoji: &'static str,
    pub logo_primary: [&'static str; 7],
    pub logo_secondary: &'static str,
    pub crate_version: &'static str,
    pub metadata_key: &'static str,
}

const THEME_DEFAULT: BannerTheme = BannerTheme {
    emoji: "🦀",
    logo_primary: [
        ansi_rgb_bold!(210, 15, 57),
        ansi_rgb_bold!(254, 100, 11),
        ansi_rgb_bold!(223, 142, 29),
        ansi_rgb_bold!(64, 160, 43),
        ansi_rgb_bold!(32, 159, 181),
        ansi_rgb_bold!(30, 102, 245),
        ansi_rgb_bold!(114, 135, 253),
    ],
    logo_secondary: "\x1B[38;5;254m",
    crate_version: "[1;33m",
    metadata_key: "[1;33m",
};

/// Prints the startup banner to [`Stdout`].
///
/// This function is for internal use in vexide's `#[vexide::main]` macro.
#[inline]
pub fn print() {
    const VEXIDE_VERSION: &str = "0.3.0";

    let system_version = unsafe { vexSystemVersion() }.to_be_bytes();
    let competition_status = unsafe { vexCompetitionStatus() };

    const DISABLED: u32 = 1 << 0;
    const AUTONOMOUS: u32 = 1 << 1;

    println!(
        "
{lp1}=%%%%%#-  {ls}-#%%%%-\x1B[0m{lp1}  :*%%%%%+.\x1B{cv}   {emoji} vexide {vexide_version}\x1B[0m
{lp2}  -#%%%%#-  {ls}:%-\x1B[0m{lp2}  -*%%%%#\x1B[0m       ---------------
{lp3}    *%%%%#=   -#%%%%%+\x1B[0m         ╭─\x1B{mk}🔲 VEXos:\x1B[0m {vexos_version}
{lp4}      *%%%%%+#%%%%%%%#=\x1B[0m        ├─\x1B{mk}🦀 Rust:\x1B[0m {rust_version}
{lp5}        *%%%%%%%*-+%%%%%+\x1B[0m      ├─\x1B{mk}🏆 Mode:\x1B[0m {competition_mode}
{lp6}          +%%%*:   .+###%#\x1B[0m     ├─\x1B{mk}🔋 Battery:\x1B[0m {battery}%
{lp7}           .%:\x1B[0m                 ╰─\x1B{mk}⌚ Uptime:\x1B[0m {uptime:?}
",
        lp1 = THEME_DEFAULT.logo_primary[0],
        lp2 = THEME_DEFAULT.logo_primary[1],
        lp3 = THEME_DEFAULT.logo_primary[2],
        lp4 = THEME_DEFAULT.logo_primary[3],
        lp5 = THEME_DEFAULT.logo_primary[4],
        lp6 = THEME_DEFAULT.logo_primary[5],
        lp7 = THEME_DEFAULT.logo_primary[6],
        ls = THEME_DEFAULT.logo_secondary,
        cv = THEME_DEFAULT.crate_version,
        mk = THEME_DEFAULT.metadata_key,
        emoji = THEME_DEFAULT.emoji,
        vexide_version = VEXIDE_VERSION,
        vexos_version = format_args!(
            "{}.{}.{}-r{}",
            system_version[0], system_version[1], system_version[2], system_version[3],
        ),
        battery = unsafe { vexBatteryCapacityGet() } as u8,
        rust_version = compile_time::rustc_version_str!(),
        competition_mode = if competition_status & DISABLED != 0 {
            "Disabled"
        } else if competition_status & AUTONOMOUS != 0 {
            "Autonomous"
        } else {
            "Driver"
        },
        uptime = Duration::from_micros(unsafe { vexSystemPowerupTimeGet() }),
    );
}
