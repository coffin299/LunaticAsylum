use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct AppState {
    pub root: Option<PathBuf>,
    children: HashMap<String, Child>,
    intentional_stop: HashSet<String>,
    pub crash_restart: HashMap<String, bool>,
    pub update_flags: HashMap<String, bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            root: None,
            children: HashMap::new(),
            intentional_stop: HashSet::new(),
            crash_restart: HashMap::new(),
            update_flags: HashMap::new(),
        }
    }
}

impl AppState {
    pub fn is_running(&mut self, id: &str) -> bool {
        if let Some(child) = self.children.get_mut(id) {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.children.remove(id);
                    false
                }
                Ok(None) => true,
                Err(_) => {
                    self.children.remove(id);
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn pid_of(&self, id: &str) -> Option<u32> {
        self.children.get(id).map(|c| c.id())
    }

    pub fn start(&mut self, id: &str, exe: &Path, cwd: &Path) -> Result<(), String> {
        if self.is_running(id) {
            return Err("already running".into());
        }
        self.intentional_stop.remove(id);

        let asylum = cwd.join(".asylum");
        std::fs::create_dir_all(&asylum).map_err(|e| e.to_string())?;
        let log_path = asylum.join("process.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| e.to_string())?;
        let log_err = log_file.try_clone().map_err(|e| e.to_string())?;

        let child = Command::new(exe)
            .current_dir(cwd)
            .args([
                "-useperfthreads",
                "-NoAsyncLoadingThread",
                "-UseMultithreadForDS",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_err))
            .spawn()
            .map_err(|e| e.to_string())?;
        self.children.insert(id.to_string(), child);
        Ok(())
    }

    pub fn stop_intentional(&mut self, id: &str) -> Result<(), String> {
        self.intentional_stop.insert(id.to_string());
        if let Some(mut child) = self.children.remove(id) {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }

    pub fn take_exited_for_restart(&mut self) -> Vec<(String, PathBuf, PathBuf)> {
        let mut to_restart = Vec::new();
        let ids: Vec<String> = self.children.keys().cloned().collect();
        for id in ids {
            if !self.crash_restart.get(&id).copied().unwrap_or(false) {
                continue;
            }
            if self.intentional_stop.contains(&id) {
                continue;
            }
            let exited = match self.children.get_mut(&id).map(|c| c.try_wait()) {
                Some(Ok(Some(_))) => true,
                Some(Err(_)) => true,
                _ => false,
            };
            if !exited {
                continue;
            }
            self.children.remove(&id);
            if let Some(root) = &self.root {
                let instance = root.join("Servers").join(&id);
                if let Some(exe) = crate::paths::find_palserver_exe(&instance) {
                    to_restart.push((id, exe, instance));
                }
            }
        }
        to_restart
    }
}

pub fn spawn_crash_monitor(state: Arc<Mutex<AppState>>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(3));
        let restart_list = {
            let Ok(mut g) = state.lock() else {
                continue;
            };
            g.take_exited_for_restart()
        };
        for (id, exe, cwd) in restart_list {
            if let Ok(mut g) = state.lock() {
                let _ = g.start(&id, &exe, &cwd);
            }
        }
    });
}
