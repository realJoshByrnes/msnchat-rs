# MSN Chat OCX Hook Status Tracker

This file tracks the functions hooked and patched within the `MsnChat45.ocx` control.

| Target Address / Symbol | Status | Description | Detour Location |
| :--- | :---: | :--- | :--- |
| `sub_3720E0A5` | 🟡 | VirtualProtect fix. Restores executable permissions (`PAGE_EXECUTE_READWRITE`) to dynamically written instructions, calls the trampoline, then restores original permissions. | [virtual_protect.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/virtual_protect.rs) |
| `sub_3721D4D3` | 🟢 | Intercepts sound index calls (`0..8`), loading files directly from mapped memory RVAs of `MsnChat45.ocx` instead of reading registry sound scheme paths. | [sound_patch.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/sound_patch.rs) |
| `sub_3721DA6C` | 🟡 | Intercepts Gatekeeper ID checks. Resolves a zeroed registry-derived GUID failure by generating a valid UUID v4 on the fly and writing it to the output parameter, then calls the trampoline. | [gatekeeper_id.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/gatekeeper_id.rs) |
| `sub_372321AE` | 🟡 | Directory Server Send. Detours outgoing commands to log them (e.g. `AUTH`, `NICK`, `FINDS`), then calls the trampoline. | [directory/send.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/directory/send.rs) |
| `sub_372327DC` | 🟡 | Directory Server Recv. Detours incoming response lines to log them, then calls the trampoline. | [directory/recv.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/directory/recv.rs) |
| `sub_3723E750` | 🟡 | Channel Server Send. Detours outgoing room messages/commands to log them, then calls the trampoline. | [channel/send.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/channel/send.rs) |
| `sub_3723EAE1` | 🟡 | Channel Server Recv. Detours incoming room responses to log them, then calls the trampoline. | [channel/recv.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/channel/recv.rs) |
| `PlaySoundA` (`winmm.dll`) | 🟢 | Intercepted to exclusively stop our Rust rodio background player when a null sound pointer is passed. | [sound_patch.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/sound_patch.rs) |
| `sub_37232EB9` | 🟢 | Socket::Create. Replaced to generate a custom Rust Socket ID and track it in our async registry. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_37232F00` | 🟢 | Socket::Close. Replaced to close the Tokio reader/writer tasks and clean up the active socket registry. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_37232F1D` | 🟢 | Socket::Connect. Bypasses native winsock connect; resolves the hostname and starts a Tokio TcpStream connection. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_37232FC2` | 🟢 | Socket::Shutdown. Shuts down reading and writing halves of the Tokio TCP connection. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_37232FDD` | 🟢 | Socket::Receive. Reads data directly from the thread-safe Tokio reader memory buffer. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_37233000` | 🟢 | Socket::Send. Writes data directly to the Tokio socket writer task channel. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |
| `sub_372329D0` | 🟢 | SocketManager::Register. Intercepts callback pointer registration to link async Tokio events to OCX handlers. | [network.rs](file:///c:/Users/jd/Desktop/MSN%20Chat%20Control/redmond-chat/src/patch/network.rs) |

## Legend

- **🟢 Fully Replaced / Implemented**: Custom Rust logic replaces the original function entirely. The original code in the OCX is suppressed (we return directly and do not call the original trampoline).
- **🟡 Partially Patched / Trampoline**: The detour intercepts execution to log data, modify arguments, or adjust execution state, but ultimately forwards execution back to the original OCX code via a trampoline.
