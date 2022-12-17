/* Copyright (C) 2020-2022 by Jacob Alexander
 *
 * This file is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This file is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this file.  If not, see <http://www.gnu.org/licenses/>.
 */

/// Logging functions
/// Handles general logging setup and other special functions
use flexi_logger::{FileSpec, Logger};
use std::env;

/// Logging setup
pub fn setup_logging() -> Result<(), std::io::Error> {
    match Logger::try_with_env_or_str("")
        .unwrap()
        .log_to_file(FileSpec::default().directory(env::temp_dir()))
        //.format(flexi_logger::colored_detailed_format)
        .format(flexi_logger::colored_default_format)
        .format_for_files(flexi_logger::colored_detailed_format)
        .rotate(
            flexi_logger::Criterion::Size(1_000_000),
            flexi_logger::Naming::Numbers,
            flexi_logger::Cleanup::KeepLogFiles(5),
        )
        .duplicate_to_stderr(flexi_logger::Duplicate::All)
        .start()
    {
        Err(msg) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Could not start logger {msg}"),
        )),
        Ok(_) => {
            info!("-------------------------- HID-IO Core starting! --------------------------");
            info!("Log location -> {:?}", env::temp_dir());
            Ok(())
        }
    }
}

/// Lite logging setup
pub fn setup_logging_lite() -> Result<(), std::io::Error> {
    match Logger::try_with_env_or_str("")
        .unwrap()
        .format(flexi_logger::colored_default_format)
        .format_for_files(flexi_logger::colored_detailed_format)
        .duplicate_to_stderr(flexi_logger::Duplicate::All)
        .start()
    {
        Err(msg) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Could not start logger {msg}"),
        )),
        Ok(_) => Ok(()),
    }
}
