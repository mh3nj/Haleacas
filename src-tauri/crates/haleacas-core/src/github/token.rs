//! GitHub personal access token storage via the OS credential/keychain
//! facilities (spec section 20: "never store tokens in plain text...
//! use the operating system credential/keychain facilities").
//!
//! The token itself **never** touches Haleacas' own SQLite database —
//! only the OS keychain (Windows Credential Manager, macOS Keychain, or
//! the Linux Secret Service via D-Bus) ever holds the actual secret.
//! `Settings.githubTokenConfigured` (a plain boolean) is the only thing
//! stored in SQLite, so the UI can show connection state without the
//! database ever containing anything sensitive.
//!
//! **Verification note, stated plainly:** this sandbox is a headless
//! Linux container with no D-Bus session bus and no Secret Service
//! provider (no gnome-keyring, no kwallet) running — `ps aux` and
//! `$DBUS_SESSION_BUS_ADDRESS` both confirm this. That means the actual
//! store/load/delete round trip **could not be live-verified in this
//! environment**, the same honest limitation as the full GUI build
//! (spec's own Windows-target reality vs. this Linux sandbox). The code
//! below is written against the `keyring` crate's real, documented API
//! (verified by reading its source directly, not from memory) and
//! compiles against real platform-specific backends — Windows Credential
//! Manager on Windows, Keychain on macOS, Secret Service on Linux — but
//! someone with a real desktop session needs to confirm the round trip
//! actually works. See STATUS.md.

use keyring::Entry;

const SERVICE: &str = "dev.haleacas.github";
const USERNAME: &str = "personal-access-token";

fn entry() -> Result<Entry, String> {
    Entry::new(SERVICE, USERNAME)
        .map_err(|e| format!("Could not access the OS credential store: {e}"))
}

/// Store (or overwrite) the GitHub token in the OS keychain.
pub fn store(token: &str) -> Result<(), String> {
    entry()?
        .set_password(token)
        .map_err(|e| format!("Could not save the token to the OS credential store: {e}"))
}

/// Load the stored token, if any. `Ok(None)` — not an error — when
/// nothing has been stored yet.
pub fn load() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Could not read the token from the OS credential store: {e}")),
    }
}

/// Remove the stored token ("Disconnect"). Also not an error if nothing
/// was stored.
pub fn delete() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Could not remove the token from the OS credential store: {e}")),
    }
}
