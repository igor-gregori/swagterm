use crate::swagger::{Operation, SwaggerSpec};

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
    pub operation: Operation,
    pub tag: String,
}

#[derive(PartialEq)]
pub enum Panel {
    Sidebar,
    Detail,
}

pub struct App {
    pub spec: SwaggerSpec,
    pub endpoints: Vec<Endpoint>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub scroll: u16,
    pub search: String,
    pub searching: bool,
    pub active_panel: Panel,
    pub quit: bool,
}

impl App {
    pub fn new(spec: SwaggerSpec) -> Self {
        let mut endpoints = Vec::new();
        for (path, methods) in &spec.paths {
            for (method, op) in methods {
                let tag = op.tags.first().cloned().unwrap_or_else(|| "default".into());
                endpoints.push(Endpoint {
                    method: method.to_uppercase(),
                    path: path.clone(),
                    operation: op.clone(),
                    tag,
                });
            }
        }
        let filtered: Vec<usize> = (0..endpoints.len()).collect();
        Self {
            spec,
            endpoints,
            filtered,
            selected: 0,
            scroll: 0,
            search: String::new(),
            searching: false,
            active_panel: Panel::Sidebar,
            quit: false,
        }
    }

    pub fn apply_filter(&mut self) {
        let query = self.search.to_lowercase();
        self.filtered = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                if query.is_empty() {
                    return true;
                }
                e.path.to_lowercase().contains(&query)
                    || e.method.to_lowercase().contains(&query)
                    || e.operation
                        .summary
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
                    || e.tag.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
        self.scroll = 0;
    }

    pub fn selected_endpoint(&self) -> Option<&Endpoint> {
        self.filtered
            .get(self.selected)
            .map(|&i| &self.endpoints[i])
    }

    pub fn next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
            self.scroll = 0;
        }
    }

    pub fn prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.filtered.len() - 1);
            self.scroll = 0;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}
