//! Which window shows which container (A4). Plain data, so it lives in the core with its tests; the shell wraps the
//! registry in a `Mutex`, builds the webviews and persists `to_saved()` under the app setting `open_windows`.
//!
//! Title rules (§10.1): a project window is titled with the project name, a workspace window with the workspace name,
//! the start screen `"Fanout Code"`. A container is shown in at most one window; start-screen windows may repeat.

use serde::{Deserialize, Serialize};

use crate::domain::{Container, ProjectId, WindowId};
use crate::ids::window_id;
use crate::{Error, Result};

/// The start screen's title.
pub const START_TITLE: &str = "Fanout Code";

/// One open window. `container: None` is the start screen (with `project_ids` empty); a project window lists its
/// one project, a workspace window its projects in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct WindowInfo {
    pub window_id: WindowId,
    pub title: String,
    pub container: Option<Container>,
    pub project_ids: Vec<ProjectId>,
}

/// The persisted shape (`open_windows`), kept apart from `WindowInfo` so it can change without touching IPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedWindow {
    pub window_id: WindowId,
    pub title: String,
    pub container: Option<Container>,
    pub project_ids: Vec<ProjectId>,
}

/// Open windows in open order, plus the highest `w-<n>` handed out. Not `Sync`; the shell wraps it in a `Mutex`.
#[derive(Debug, Default)]
pub struct WindowRegistry {
    windows: Vec<WindowInfo>,
    next: u64,
}

fn number(id: &WindowId) -> u64 {
    id.as_str().strip_prefix("w-").and_then(|n| n.parse().ok()).unwrap_or(0)
}

impl WindowRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps the saved windows whose container still exists (with their ids, titles and project ids), drops the rest
    /// (start-screen windows included), and continues numbering above the highest saved id. With nothing left it
    /// yields one start-screen window `w-1`.
    pub fn restore(&mut self, saved: Vec<SavedWindow>, exists: impl Fn(&Container) -> bool) -> Vec<WindowInfo> {
        self.next = self
            .next
            .max(saved.iter().map(|w| number(&w.window_id)).max().unwrap_or(0));
        for w in saved {
            let Some(container) = w.container else { continue };
            if !exists(&container) || self.find(&container).is_some() || self.get(&w.window_id).is_some() {
                continue;
            }
            self.windows.push(WindowInfo {
                window_id: w.window_id,
                title: w.title,
                container: Some(container),
                project_ids: w.project_ids,
            });
        }
        if self.windows.is_empty() {
            let id = WindowId::parse(&window_id(1)).expect("w-1 is a valid window id");
            self.next = self.next.max(1);
            self.windows.push(WindowInfo {
                window_id: id,
                title: START_TITLE.into(),
                container: None,
                project_ids: vec![],
            });
        }
        self.list()
    }

    /// Registers a new window, or returns the one already showing `container` with `true` (one window per container,
    /// §4.1). `open(None, …)` always registers a new start-screen window.
    pub fn open(
        &mut self,
        container: Option<Container>,
        title: String,
        project_ids: Vec<ProjectId>,
    ) -> (WindowInfo, bool) {
        if let Some(existing) = container
            .as_ref()
            .and_then(|c| self.find(c))
            .and_then(|id| self.get(&id))
        {
            return (existing, true);
        }
        self.next += 1;
        let id = WindowId::parse(&window_id(self.next)).expect("w-<n> is a valid window id");
        let info = WindowInfo {
            window_id: id,
            title,
            container,
            project_ids,
        };
        self.windows.push(info.clone());
        (info, false)
    }

    /// Points window `label` at `container` (Plan 10's workspace conversions). A project sets `project_ids` to it; a
    /// workspace keeps the current `project_ids` and title until `set_details`.
    pub fn set_container(&mut self, label: &WindowId, container: Container) -> Result<WindowInfo> {
        if self.find(&container).is_some_and(|id| &id != label) {
            return Err(Error::Precondition(
                "That project or workspace is already open in another window.".into(),
            ));
        }
        let w = self.entry(label)?;
        if let Container::Project { id } = &container {
            w.project_ids = vec![id.clone()];
        }
        w.container = Some(container);
        Ok(w.clone())
    }

    /// Retitles window `label` and re-lists its projects.
    pub fn set_details(&mut self, label: &WindowId, title: String, project_ids: Vec<ProjectId>) -> Result<WindowInfo> {
        let w = self.entry(label)?;
        w.title = title;
        w.project_ids = project_ids;
        Ok(w.clone())
    }

    pub fn close(&mut self, id: &WindowId) {
        self.windows.retain(|w| &w.window_id != id);
    }

    pub fn get(&self, id: &WindowId) -> Option<WindowInfo> {
        self.windows.iter().find(|w| &w.window_id == id).cloned()
    }

    /// Open order.
    pub fn list(&self) -> Vec<WindowInfo> {
        self.windows.clone()
    }

    /// The window showing `container`, if any.
    pub fn find(&self, container: &Container) -> Option<WindowId> {
        self.windows
            .iter()
            .find(|w| w.container.as_ref() == Some(container))
            .map(|w| w.window_id.clone())
    }

    /// The next window in open order, wrapping; `None` when `id` is unknown or the only window.
    pub fn next_after(&self, id: &WindowId) -> Option<WindowId> {
        let i = self.windows.iter().position(|w| &w.window_id == id)?;
        if self.windows.len() < 2 {
            return None;
        }
        Some(self.windows[(i + 1) % self.windows.len()].window_id.clone())
    }

    pub fn to_saved(&self) -> Vec<SavedWindow> {
        self.windows
            .iter()
            .map(|w| SavedWindow {
                window_id: w.window_id.clone(),
                title: w.title.clone(),
                container: w.container.clone(),
                project_ids: w.project_ids.clone(),
            })
            .collect()
    }

    fn entry(&mut self, label: &WindowId) -> Result<&mut WindowInfo> {
        self.windows
            .iter_mut()
            .find(|w| &w.window_id == label)
            .ok_or_else(|| Error::Invalid("Unknown window.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectId, WorkspaceId};
    use crate::Error;

    fn sw(id: &str, container: Option<Container>) -> SavedWindow {
        let project_ids = match &container {
            Some(Container::Project { id }) => vec![id.clone()],
            _ => vec![],
        };
        SavedWindow {
            window_id: WindowId::parse(id).unwrap(),
            title: "saved".into(),
            container,
            project_ids,
        }
    }

    #[test]
    fn restore_drops_vanished_containers_and_keeps_ids() {
        let p = ProjectId::parse("p-1").unwrap();
        let gone = ProjectId::parse("p-2").unwrap();
        let saved = vec![
            sw("w-2", Some(Container::Project { id: p.clone() })),
            sw("w-5", Some(Container::Project { id: gone })),
        ];
        let mut r = WindowRegistry::new();
        let open = r.restore(saved, |c| c == &Container::Project { id: p.clone() });
        assert_eq!(open.iter().map(|w| w.window_id.as_str()).collect::<Vec<_>>(), ["w-2"]);
        assert_eq!(open[0].project_ids, std::slice::from_ref(&p));
        assert_eq!(open[0].title, "saved");
        let (next, _) = r.open(None, "Fanout Code".into(), vec![]);
        assert_eq!(
            next.window_id.as_str(),
            "w-6",
            "the counter continues above the highest restored id"
        );
    }

    #[test]
    fn restore_with_nothing_yields_the_start_screen_window() {
        let mut r = WindowRegistry::new();
        let open = r.restore(vec![], |_| false);
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].window_id.as_str(), "w-1");
        assert_eq!(open[0].container, None);
        assert_eq!(open[0].title, "Fanout Code");
        assert!(open[0].project_ids.is_empty());
    }

    #[test]
    fn opening_a_container_twice_reuses_its_window() {
        let mut r = WindowRegistry::new();
        let c = Container::Workspace {
            id: WorkspaceId::parse("ws-a").unwrap(),
        };
        let (first, existed) = r.open(Some(c.clone()), "Acme".into(), vec![]);
        assert!(!existed);
        let (again, existed) = r.open(Some(c.clone()), "Acme".into(), vec![]);
        assert!(existed);
        assert_eq!(first.window_id, again.window_id);
        r.close(&first.window_id);
        assert!(r.find(&c).is_none());
        assert_eq!(r.to_saved().len(), 0);
    }

    #[test]
    fn set_container_converts_a_window_and_keeps_one_window_per_container() {
        let mut r = WindowRegistry::new();
        let p = ProjectId::parse("p-1").unwrap();
        let q = ProjectId::parse("p-2").unwrap();
        let (start, _) = r.open(None, "Fanout Code".into(), vec![]);
        let (proj, _) = r.open(
            Some(Container::Project { id: p.clone() }),
            "api".into(),
            vec![p.clone()],
        );
        let ws = Container::Workspace {
            id: WorkspaceId::parse("ws-a").unwrap(),
        };
        let w = r.set_container(&proj.window_id, ws.clone()).unwrap();
        assert_eq!(w.container, Some(ws.clone()));
        assert_eq!(w.project_ids, std::slice::from_ref(&p));
        assert_eq!(r.find(&ws), Some(proj.window_id.clone()));
        assert!(r.find(&Container::Project { id: p.clone() }).is_none());
        assert!(matches!(
            r.set_container(&start.window_id, ws),
            Err(Error::Precondition(_))
        ));
        assert!(matches!(
            r.set_container(&WindowId::parse("w-99").unwrap(), Container::Project { id: q.clone() }),
            Err(Error::Invalid(_))
        ));
        let w = r.set_details(&proj.window_id, "Acme".into(), vec![p, q]).unwrap();
        assert_eq!(w.title, "Acme");
        assert_eq!(w.project_ids.len(), 2);
        assert_eq!(r.to_saved()[1].title, "Acme");
    }

    #[test]
    fn next_after_cycles_in_open_order() {
        let mut r = WindowRegistry::new();
        let a = r.open(None, "Fanout Code".into(), vec![]).0.window_id;
        let b = r.open(None, "Fanout Code".into(), vec![]).0.window_id;
        assert_eq!(r.next_after(&a), Some(b.clone()));
        assert_eq!(r.next_after(&b), Some(a.clone()));
        assert_eq!(r.get(&b).unwrap().title, "Fanout Code");
        r.close(&b);
        assert_eq!(r.next_after(&a), None);
        assert!(r.get(&b).is_none());
    }
}
