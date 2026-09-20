use super::{
    layout::{
        PaneNode,
        SplitDirection,
    },
    pane::Pane,
};

use crate::terminal::Terminal;

pub struct Tab {
    root: PaneNode,
    focused_pane: usize,
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
}
