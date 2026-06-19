use crate::swagger::{Operation, SwaggerSpec};
use std::collections::HashSet;

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

#[derive(Debug, Clone)]
pub enum SidebarItem {
    Tag(String),
    Endpoint(usize), // index into endpoints vec
}

pub struct App {
    pub spec: SwaggerSpec,
    pub endpoints: Vec<Endpoint>,
    pub filtered: Vec<usize>,
    pub sidebar_items: Vec<SidebarItem>,
    pub selected: usize,
    pub scroll: u16,
    pub detail_height: u16,
    pub sidebar_scroll: u16,
    pub sidebar_height: u16,
    pub collapsed_tags: HashSet<String>,
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
        let mut app = Self {
            spec,
            endpoints,
            filtered,
            sidebar_items: Vec::new(),
            selected: 0,
            scroll: 0,
            detail_height: 0,
            sidebar_scroll: 0,
            sidebar_height: 0,
            collapsed_tags: HashSet::new(),
            search: String::new(),
            searching: false,
            active_panel: Panel::Sidebar,
            quit: false,
        };
        app.rebuild_sidebar();
        app
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
        self.rebuild_sidebar();
        if self.selected >= self.sidebar_items.len() {
            self.selected = 0;
        }
        self.scroll = 0;
    }

    pub fn rebuild_sidebar(&mut self) {
        self.sidebar_items.clear();
        let mut current_tag = String::new();
        for &idx in &self.filtered {
            let ep = &self.endpoints[idx];
            if ep.tag != current_tag {
                current_tag = ep.tag.clone();
                self.sidebar_items.push(SidebarItem::Tag(current_tag.clone()));
            }
            if !self.collapsed_tags.contains(&ep.tag) {
                self.sidebar_items.push(SidebarItem::Endpoint(idx));
            }
        }
    }

    pub fn selected_endpoint(&self) -> Option<&Endpoint> {
        match self.sidebar_items.get(self.selected) {
            Some(SidebarItem::Endpoint(idx)) => Some(&self.endpoints[*idx]),
            _ => None,
        }
    }

    pub fn toggle_tag(&mut self) {
        if let Some(SidebarItem::Tag(tag)) = self.sidebar_items.get(self.selected).cloned() {
            if self.collapsed_tags.contains(&tag) {
                self.collapsed_tags.remove(&tag);
            } else {
                self.collapsed_tags.insert(tag);
            }
            self.rebuild_sidebar();
            if self.selected >= self.sidebar_items.len() {
                self.selected = self.sidebar_items.len().saturating_sub(1);
            }
        }
    }

    pub fn next(&mut self) {
        if !self.sidebar_items.is_empty() {
            self.selected = (self.selected + 1) % self.sidebar_items.len();
            self.scroll = 0;
            self.adjust_sidebar_scroll();
        }
    }

    pub fn prev(&mut self) {
        if !self.sidebar_items.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.sidebar_items.len() - 1);
            self.scroll = 0;
            self.adjust_sidebar_scroll();
        }
    }

    pub fn adjust_sidebar_scroll(&mut self) {
        if self.sidebar_height == 0 {
            return;
        }
        let h = self.sidebar_height as usize;
        if self.selected < self.sidebar_scroll as usize {
            self.sidebar_scroll = self.selected as u16;
        } else if self.selected >= self.sidebar_scroll as usize + h {
            self.sidebar_scroll = (self.selected - h + 1) as u16;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn page_down(&mut self) {
        let step = self.detail_height.max(1);
        self.scroll = self.scroll.saturating_add(step);
    }

    pub fn page_up(&mut self) {
        let step = self.detail_height.max(1);
        self.scroll = self.scroll.saturating_sub(step);
    }
}
