// msnchat-rs
// Copyright (C) 2025 Joshua Byrnes
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::PCSTR;

/// Called by the OCX to determine if a TLD is allowed.
/// The buggy TLD is the last two parts of a domain, irrelevant of actual TLD.
///
/// The OCX has a list of "known" TLDs that are allowed. If the domain
/// being checked does not have one of these TLDs, the OCX will call
/// this function to determine if the unknown TLD is allowed.
///
/// The function should return 1 if the TLD is allowed, and 0 if it is
/// not.
pub extern "cdecl" fn check_buggy_tld_is_allowed(buggy_tld: PCSTR) -> i32 {
    println!("Approved (buggy) TLD from OCX: {}", unsafe { buggy_tld.to_string().unwrap() });
    return 1;
}