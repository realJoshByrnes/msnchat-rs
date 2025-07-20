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

// AUTHORS NOTE:
// This file doesn't do anything, currently.
// It was creating during a failed attempt to allow coloured emoji within the OCX.
//
// FWIW: You'll need the latest RichEdit from (x86 install of) Microsoft Office...
// C:\Program Files (x86)\Microsoft Office\root\vfs\ProgramFilesCommonX86\Microsoft Shared\Office16\RICHED20.DLL
//
// - JD 20/07/2025

use std::os::raw::c_void;
use windows::core::{s, PCSTR};

use crate::patch::{
    msnchat45::reloc::PatchContext,
    utils::{make_call_rel32, patch_bytes},
};

const MSFTEDIT_FILE: PCSTR = s!("RICHED20.dll");
const MSFTEDIT_CLASS_A: PCSTR = s!("RichEditD2D");
// windows::Win32::UI::Controls::RichEdit::MSFTEDIT_CLASS contains the wide variant.

pub fn init(ctx: &PatchContext) {
    unsafe {
        let patch = push_instr(MSFTEDIT_FILE);
        patch_bytes(ctx.adjust(0x372249F9), &patch);
        patch_bytes(ctx.adjust(0x37226491), &patch);

        let patch = mov_eax_instr(MSFTEDIT_CLASS_A);
        patch_bytes(ctx.adjust(0x37223438), &patch);
        patch_bytes(ctx.adjust(0x37224B43), &patch);
        patch_bytes(ctx.adjust(0x37225944), &patch);
        patch_bytes(ctx.adjust(0x37226504), &patch);

        // MSNChatRichEdit4 / WM_CREATE
        make_call_rel32(ctx.adjust(0x37224977), sub_37223f13_trampoline as usize);
    }
}

fn push_instr(ptr: PCSTR) -> [u8; 5] {
    let addr = ptr.0 as usize as u32;
    let mut instr = [0x68, 0, 0, 0, 0];
    instr[1..].copy_from_slice(&addr.to_le_bytes());
    instr
}

fn mov_eax_instr(ptr: PCSTR) -> [u8; 5] {
    let addr = ptr.0 as usize as u32;
    let mut instr = [0xB8, 0, 0, 0, 0];
    instr[1..].copy_from_slice(&addr.to_le_bytes());
    instr
}

type Sub37223F13Fn = unsafe extern "thiscall" fn(
    this: *mut c_void,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
) -> i32;

unsafe extern "thiscall" fn sub_37223f13_trampoline(
    this: *mut c_void,
    a2: i32,
    a3: i32,
    a4: i32,
    a5: i32,
) -> i32 {
    let ctx = PatchContext::get().unwrap();
    unsafe {
        let original: Sub37223F13Fn = std::mem::transmute(ctx.adjust(0x37223F13));
        let result = original(this, a2, a3, a4, a5);
        // let hwnd_ptr = (this as *const HWND).add(34);
        result
    }
}