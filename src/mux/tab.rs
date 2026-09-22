use super::{
    layout::{
        FocusDirection,
        PaneNode,
        PaneRect,
        SeparatorRect,
        SplitDirection,
    },
    pane::Pane,
};

use crate::terminal::Terminal;

pub struct Tab {
    root: PaneNode,
    focused_pane: usize,
    zoomed_pane: Option<usize>,
}

impl Tab {
    pub fn new(
        columns: usize,
        rows: usize,
    ) -> Self {
        let root =
            PaneNode::new_pane(
                columns,
                rows,
            );
        
        let focused_pane =
            root.first_pane().id();
        
        Self {
            root,
            focused_pane,
            zoomed_pane: None,
        }
    }

    pub fn terminal(&self) -> &Terminal {
        self.active_pane().terminal()
    }

    pub fn terminal_mut(
        &mut self,
    ) -> &mut Terminal {
        self.active_pane_mut().terminal_mut()
    }

    pub fn write(
        &mut self,
        text: &str,
    ) {
        self.active_pane_mut().write(text);
    }


    pub fn process_output(
        &mut self,
    ) -> bool {
        self.root.process_output()
    }

    pub fn resize(
        &mut self,
        columns: usize,
        rows: usize,
    ) {
        self.root.resize(
            columns,
            rows,
        );
    }

    pub fn root(&self) -> &PaneNode {
        &self.root
    }

    pub fn root_mut(
        &mut self,
    ) -> &mut PaneNode {
        &mut self.root
    }

    pub fn active_pane(&self) -> &Pane {
        self.root
            .find_pane(self.focused_pane)
            .expect(
                "Focused pane does not exist"
            )
    }

    pub fn active_pane_mut(
        &mut self,
    ) -> &mut Pane {
        self.root
            .find_pane_mut(self.focused_pane)
            .expect(
                "Focused pane does not exist"
            )
    }

    pub fn split_active(
        &mut self,
        direction: SplitDirection,
    ) {
        let columns = self.active_pane()
        .terminal().width();

        let rows = self.active_pane()
        .terminal().height();

        if let Some(new_pane_id) =
            self.root.split_pane(
                self.focused_pane,
                direction,
                columns,
                rows,
            ) {
                self.focused_pane = new_pane_id;
            }
    }

    pub fn for_each_pane<F>(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        callback: &mut F,
    )
    where
        F: FnMut(
            &Pane,
            u32,
            u32,
            u32,
            u32,
        ),
    {
        self.root.for_each_pane(
            x,
            y,
            width,
            height,
            callback,
        );
    }

    pub fn for_each_visible_pane<F>(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        callback: &mut F,
    )
    where
        F: FnMut(
            &Pane,
            u32,
            u32,
            u32,
            u32,
        ),
    {
        if let Some(pane_id) = self.zoomed_pane {
            if let Some(pane) = self.root.find_pane(pane_id) {
                callback(
                    pane,
                    x,
                    y,
                    width,
                    height,
                );
            }

            return;
        }

        self.root.for_each_pane(
            x,
            y,
            width,
            height,
            callback,
        );
    }

    pub fn focused_pane_id(
        &self,
    ) -> usize {
        self.focused_pane
    }

    pub fn focus_next_pane(
        &mut self,
    ) {
        let mut pane_ids = Vec::new();

        self.root.collect_pane_ids(
            &mut pane_ids,
        );

        if pane_ids.len() <= 1{
            return;
        }

        let current_index = pane_ids.iter()
        .position(|id| {
            *id == self.focused_pane
        })
        .unwrap_or(0);

        let next_index = (current_index + 1) % pane_ids.len();

        self.focused_pane = pane_ids[next_index];
    }

    pub fn focus_previous_pane(
        &mut self,
    ) {
        let mut pane_ids = Vec::new();
        self.root.collect_pane_ids(
            &mut pane_ids,
        );

        if pane_ids.len() <= 1 {
            return;
        }

        let current_index = pane_ids
            .iter()
            .position(|id| {
                *id == self.focused_pane
            })
            .unwrap_or(0);

        let previous_index =
            if current_index == 0 {
                pane_ids.len() - 1
            } else {
                current_index - 1
            };

        self.focused_pane = pane_ids[previous_index];
    }

    pub fn close_active_pane(
        &mut self,
    ) {
        let mut pane_ids = Vec::new();

        self.root.collect_pane_ids(&mut pane_ids, );

        if pane_ids.len() <= 1 {
            return;
        }

        let current_index =
            pane_ids 
                .iter()
                .position(|id| {
                    *id == self.focused_pane
                })
                .unwrap_or(0);

        let next_focus = 
                if current_index > 0 {
                    pane_ids[current_index - 1]
                } else {
                    pane_ids[1]
                };

        if self.root.remove_pane(
            self.focused_pane
        ) {
            self.focused_pane = next_focus;
        }
    }

    pub fn for_each_separator<F>(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        callback: &mut F,
    )
    where
        F: FnMut(SeparatorRect),
    {
        self.root.for_each_separator(
            x,
            y,
            width,
            height,
            callback,
        );
    }

    pub fn pane_rects(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Vec<PaneRect> {
        let mut rects = Vec::new();
        self.root.collect_pane_rects(
            x,
            y,
            width,
            height,
            &mut rects,
        );

        rects
    }

    pub fn focus_direction(
        &mut self,
        direction: FocusDirection,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        let rects = 
            self.pane_rects(
                x,
                y,
                width,
                height,
            );

        let Some(current) =
            rects
                .iter()
                .find(|rect| {
                    rect.pane_id == self.focused_pane
                })
        else {
            return;
        };

        let current_center_x = current.x as i64 + current.width as i64 / 2;
        let current_center_y = current.y as i64 + current.height as i64 / 2;

        let mut best_pane = None;
        let mut best_distance = i64::MAX;

        for candidate in &rects {
            if candidate.pane_id == self.focused_pane {
                continue;
            }

            let candidate_center_x = candidate.x as i64 + candidate.width as i64 / 2;
            let candidate_center_y = candidate.y as i64 + candidate.height as i64 / 2;

            let dx = candidate_center_x - current_center_x;
            let dy = candidate_center_y - current_center_y;

            let valid_direction = match direction {
                FocusDirection::Left => dx < 0,
                FocusDirection::Right => dx > 0,
                FocusDirection::Up => dy < 0,
                FocusDirection::Down => dy > 0,
            };

            if !valid_direction {
                continue;
            }

            let distance = dx * dx + dy * dy;

            if distance < best_distance {
                best_distance = distance;
                best_pane = Some(candidate.pane_id);
            }
        }

        if let Some(pane_id) = best_pane {
            self.focused_pane = pane_id;
        }
    }

    pub fn focus_at_position(
        &mut self,
        mouse_x: f64,
        mouse_y: f64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> bool {
        let rects = self.pane_rects(
            x,
            y,
            width,
            height,
        );

        for rect in rects {
            let left = rect.x as f64;
            let right = (rect.x + rect.width) as f64;
            let top = rect.y as f64;
            let bottom = (rect.y + rect.height) as f64;
            let inside = 
                mouse_x >= left
                && mouse_x < right
                && mouse_y >= top
                && mouse_y < bottom;

            if inside {
                self.focused_pane = rect.pane_id;
                return true;
            }
        }

        false
    }

    pub fn separator_at_position(
        &self,
        mouse_x: f64,
        mouse_y: f64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Option<SeparatorRect> {
        let separators = self.root.separator_rects(
            x,
            y,
            width,
            height,
        );

        for separator in separators {
            let hit_padding = 4.0;

            let left = separator.x as f64 - hit_padding;
            let right = (separator.x + separator.width) as f64 + hit_padding;
            let top = separator.y as f64 - hit_padding;
            let bottom = (separator.y + separator.height) as f64 + hit_padding;

            let inside =
                mouse_x >= left
                && mouse_x < right
                && mouse_y >= top
                && mouse_y < bottom;

            if inside {
                return Some(separator);
            }
        }

        None
    }

    pub fn resize_split_at_position(
        &mut self,
        split_id: usize,
        mouse_x: f64,
        mouse_y: f64,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> bool {
        let Some(split) = self.root.find_split_rect(
            split_id,
            x,
            y,
            width,
            height,
        )

        else {
            return false;
        };

        let new_ratio = match split.direction {
            SplitDirection::Vertical => {
                if split.width == 0 {
                    return false;
                }

                (
                    (mouse_x - split.x as f64)
                    / split.width as f64
                ) as f32
            }

            SplitDirection::Horizontal => {
                if split.height == 0 {
                    return false;
                }

                (
                    (mouse_y - split.y as f64)
                    / split.height as f64
                ) as f32
            }
        };

        self.root.set_split_ratio(
            split_id,
            new_ratio,
        )
    }

    pub fn toggle_zoom(&mut self) {
        if self.zoomed_pane.is_some() {
            self.zoomed_pane = None;
        } else {
            self.zoomed_pane = Some(self.focused_pane);
        }
    }

    pub fn zoomed_pane(
        &self,
    ) -> Option<usize> {
        self.zoomed_pane
    }
}
