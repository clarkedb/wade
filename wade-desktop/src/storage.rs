//! Settings storage: a file in the user's config directory
//! (docs/platforms.md#desktop-wade-desktop).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use wade_core::Settings;

/// Where settings are kept between runs.
pub struct Storage {
    path: PathBuf,
}

const RETRY_DELAY: Duration = Duration::from_secs(2);

pub struct SettingsSaver<'a> {
    storage: &'a Storage,
    pending: Option<Settings>,
    retry_at: Option<Instant>,
}

impl<'a> SettingsSaver<'a> {
    pub fn new(storage: &'a Storage) -> Self {
        Self {
            storage,
            pending: None,
            retry_at: None,
        }
    }

    pub fn save(&mut self, settings: Settings) {
        self.pending = Some(settings);
        self.attempt();
    }

    pub fn retry_if_due(&mut self) {
        if self.retry_at.is_some_and(|at| Instant::now() >= at) {
            self.attempt();
        }
    }

    pub fn flush(&mut self, unsaved: Option<Settings>) {
        if let Some(settings) = unsaved {
            self.pending = Some(settings);
        }
        if self.pending.is_some() {
            self.attempt();
        }
    }

    fn attempt(&mut self) {
        let Some(settings) = self.pending else {
            return;
        };
        match self.storage.save(settings) {
            Ok(()) => {
                self.pending = None;
                self.retry_at = None;
            }
            Err(error) => {
                eprintln!(
                    "{}: settings not saved: {error}",
                    self.storage.path().display()
                );
                self.retry_at = Some(Instant::now() + RETRY_DELAY);
            }
        }
    }
}

impl Storage {
    /// The settings file in the user's config directory, or `None` if there
    /// is no such directory.
    pub fn in_config_dir() -> Option<Storage> {
        dirs::config_dir().map(|dir| Storage::at(dir.join("wade").join("settings")))
    }

    pub fn at(path: PathBuf) -> Storage {
        Storage { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The stored settings. A missing file gives the defaults, and so does an
    /// unreadable or corrupt one, with a warning.
    pub fn load(&self) -> Settings {
        match fs::read(&self.path) {
            Ok(bytes) => Settings::try_decode(&bytes).unwrap_or_else(|| {
                eprintln!(
                    "{}: not valid settings, so using the defaults",
                    self.path.display()
                );
                Settings::DEFAULT
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::DEFAULT,
            Err(error) => {
                eprintln!(
                    "{}: unreadable, so using the defaults: {error}",
                    self.path.display()
                );
                Settings::DEFAULT
            }
        }
    }

    /// Store `settings`. They are written beside the file and renamed over it,
    /// so quitting mid-write never leaves it half written.
    pub fn save(&self, settings: Settings) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let partial = self.path.with_extension("partial");
        fs::write(&partial, settings.encode())?;
        fs::rename(&partial, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use wade_core::view::ColorMode;

    use super::*;

    /// A fresh directory, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!("wade-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Scratch(dir)
        }

        fn storage(&self) -> Storage {
            Storage::at(self.0.join("wade").join("settings"))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn saved_settings_load_back_in_a_new_directory() {
        let scratch = Scratch::new("round-trip");
        let storage = scratch.storage();
        assert_eq!(storage.load(), Settings::DEFAULT);
        let mono = Settings::DEFAULT.with_color(ColorMode::Mono);
        storage.save(mono).unwrap();
        assert_eq!(storage.load(), mono);
        storage.save(Settings::DEFAULT).unwrap();
        assert_eq!(storage.load(), Settings::DEFAULT);
    }

    #[test]
    fn a_corrupt_or_unreadable_file_gives_the_defaults() {
        let scratch = Scratch::new("corrupt");
        let storage = scratch.storage();
        storage.save(Settings::DEFAULT.with_chime(false)).unwrap();
        fs::write(&storage.path, b"not settings").unwrap();
        assert_eq!(storage.load(), Settings::DEFAULT);
        fs::remove_file(&storage.path).unwrap();
        fs::create_dir(&storage.path).unwrap();
        assert_eq!(storage.load(), Settings::DEFAULT);
    }

    #[test]
    fn a_failed_save_retries_the_latest_settings_after_recovery() {
        let scratch = Scratch::new("retry");
        fs::create_dir_all(&scratch.0).unwrap();
        let blocked_dir = scratch.0.join("wade");
        fs::write(&blocked_dir, b"blocking file").unwrap();
        let storage = scratch.storage();
        let mut saver = SettingsSaver::new(&storage);
        let mono = Settings::DEFAULT.with_color(ColorMode::Mono);
        saver.save(mono);
        assert_eq!(saver.pending, Some(mono));
        assert_eq!(storage.load(), Settings::DEFAULT);

        let latest = mono.with_chime(false);
        saver.save(latest);
        fs::remove_file(blocked_dir).unwrap();
        saver.retry_at = Some(Instant::now());
        saver.retry_if_due();
        assert_eq!(storage.load(), latest);
        assert_eq!(saver.pending, None);
        assert_eq!(saver.retry_at, None);
    }

    #[test]
    fn shutdown_flushes_a_failed_save() {
        let scratch = Scratch::new("flush");
        fs::create_dir_all(&scratch.0).unwrap();
        let blocked_dir = scratch.0.join("wade");
        fs::write(&blocked_dir, b"blocking file").unwrap();
        let storage = scratch.storage();
        let mut saver = SettingsSaver::new(&storage);
        let changed = Settings::DEFAULT.with_chime(false);
        saver.save(changed);
        fs::remove_file(blocked_dir).unwrap();
        saver.flush(None);
        assert_eq!(storage.load(), changed);
        saver.flush(None);
        assert_eq!(saver.pending, None);
    }
}
