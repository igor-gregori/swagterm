use crate::swagger::{ApiSpec, Operation};
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

#[derive(PartialEq)]
pub enum AppMode {
    Browse,
    TryIt,
}

pub struct TryItState {
    pub param_values: Vec<(String, String, String)>, // (name, location, value)
    pub body: String,
    pub selected_field: usize,
    pub editing: bool,
    pub response: Option<HttpResponse>,
    pub error: Option<String>,
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub struct App {
    pub spec: ApiSpec,
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
    pub mode: AppMode,
    pub try_it: Option<TryItState>,
    pub quit: bool,
}

impl App {
    pub fn new(spec: ApiSpec) -> Self {
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
            mode: AppMode::Browse,
            try_it: None,
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

    pub fn enter_try_it(&mut self) {
        let Some(ep) = self.selected_endpoint() else { return };
        let params: Vec<(String, String, String)> = ep
            .operation
            .parameters
            .iter()
            .filter(|p| p.location != "body")
            .map(|p| (p.name.clone(), p.location.clone(), String::new()))
            .collect();
        let has_body = ep.operation.parameters.iter().any(|p| p.location == "body");
        self.try_it = Some(TryItState {
            param_values: params,
            body: if has_body { "{}".into() } else { String::new() },
            selected_field: 0,
            editing: false,
            response: None,
            error: None,
        });
        self.mode = AppMode::TryIt;
        self.scroll = 0;
    }

    pub fn exit_try_it(&mut self) {
        self.mode = AppMode::Browse;
        self.try_it = None;
        self.scroll = 0;
    }

    pub fn execute_request(&mut self) {
        let Some(ep) = self.selected_endpoint().cloned() else { return };
        let Some(state) = &self.try_it else { return };

        // Build URL with path params substituted
        let mut path = ep.path.clone();
        let mut query_params: Vec<(String, String)> = Vec::new();

        for (name, location, value) in &state.param_values {
            match location.as_str() {
                "path" => {
                    path = path.replace(&format!("{{{name}}}"), value);
                }
                "query" => {
                    if !value.is_empty() {
                        query_params.push((name.clone(), value.clone()));
                    }
                }
                _ => {}
            }
        }

        let mut url = format!("{}{}", self.spec.base_url, path);
        if !query_params.is_empty() {
            let qs: Vec<String> = query_params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            url = format!("{url}?{}", qs.join("&"));
        }

        let body = state.body.clone();
        let method = ep.method.clone();

        // Execute request
        let result = Self::do_request(&method, &url, &body);
        let state = self.try_it.as_mut().unwrap();
        match result {
            Ok(resp) => {
                state.response = Some(resp);
                state.error = None;
            }
            Err(e) => {
                state.error = Some(e);
                state.response = None;
            }
        }
        self.scroll = 0;
    }

    fn do_request(method: &str, url: &str, body: &str) -> Result<HttpResponse, String> {
        let request = match method {
            "GET" => ureq::get(url),
            "POST" => ureq::post(url),
            "PUT" => ureq::put(url),
            "DELETE" => ureq::delete(url),
            "PATCH" => ureq::patch(url),
            "HEAD" => ureq::head(url),
            _ => return Err(format!("Unsupported method: {method}")),
        };

        let response = if matches!(method, "POST" | "PUT" | "PATCH") && !body.is_empty() {
            request
                .set("Content-Type", "application/json")
                .send_string(body)
        } else {
            request.call()
        };

        match response {
            Ok(resp) => {
                let status = resp.status();
                let headers: Vec<(String, String)> = resp
                    .headers_names()
                    .iter()
                    .filter_map(|name| {
                        resp.header(name).map(|v| (name.clone(), v.to_string()))
                    })
                    .collect();
                let body = resp.into_string().unwrap_or_default();
                Ok(HttpResponse { status, headers, body })
            }
            Err(ureq::Error::Status(code, resp)) => {
                let headers: Vec<(String, String)> = resp
                    .headers_names()
                    .iter()
                    .filter_map(|name| {
                        resp.header(name).map(|v| (name.clone(), v.to_string()))
                    })
                    .collect();
                let body = resp.into_string().unwrap_or_default();
                Ok(HttpResponse { status: code, headers, body })
            }
            Err(e) => Err(format!("Request failed: {e}")),
        }
    }
}
