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

/// Called by the Chat Control to determine if a domain is allowed.
///
/// The Chat Control has a list of "known" TLDs that are allowed by default:
/// * msn.com
/// * moonport.com
/// * microsoft.com
/// * msn-int.com
///
/// The function should return true if the domain is allowed, and false if it is
/// not.
// TODO: Also used by sub_3722A0F2

pub extern "cdecl" fn is_allowed_domain(buggy_tld: PCSTR) -> bool {
    #[cfg(debug_assertions)]
    println!("Approved domain: {}", unsafe {
        buggy_tld.to_string().unwrap()
    });
    return true;
}
