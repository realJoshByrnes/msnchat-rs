# msnchat-rs

**Pure Rust ActiveX container for the MSN Chat Control**

![image](https://github.com/user-attachments/assets/c803e090-9a1c-4cb7-b895-4b4a88eeef02)


`msnchat-rs` is a modern reimplementation and preservation effort for the legacy
**MSN Chat Control** (specifically `MSNChat45.ocx`, last compiled on October 25,
2003). Over the years, changes in the Windows ecosystem and the deprecation of
Internet Explorer have rendered parts of the original control inoperable.

This project aims to restore and enhance the chat control so it can run reliably
on modern versions of Microsoft Windows. In doing so, it preserves a piece of
online communication history—while also unlocking new extensibility that the
original never envisioned.

## 🔧 Key Features & Goals

- 🧱 **Preserve** compatibility with the original chat control and its known quirks
- 🚀 **Enhance** functionality by introducing modern patches and extensibility hooks
- 💻 **Support** execution on contemporary Windows systems, post-Internet Explorer

## ⚙️ Patched Behavior

- ✅ Modified `CTCP VERSION` to reply to all users
- ✅ Enhanced `/version` command to display msnchat-rs version
- ✅ Socket creation logic updated for dual-stack IPv6
- ✅ Server field patched to accept IPv6 hostnames
- ✅ Introduced internal **command hook system** for user-defined commands
- ✅ Patched `WhisperContent` to accept content from any origin
- ✅ Updated `ResDLL` caching path to `%TEMP%/msnchat-rs.cache/`
- ✅ Allowed loading of `ResDLL` assets from arbitrary domains
- ✅ Re-enabled automatic URL launching via default browser
- ✅ Modified `CTCP VERSION` reply to include project name and version

---

📝 *This project exists to honor the weird and wonderful quirks of MSN Chat
while enabling its resurrection in today’s world. Contributions, feedback, and
nostalgic stories are always welcome.*

**Disclaimer**: MSN and Microsoft are registered trademarks of Microsoft
Corporation. This project is not affiliated with or endorsed by Microsoft in any
way.
