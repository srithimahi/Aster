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
}

impl Tab {
    pub fn new(
        columns: usize,
        rows: usize,
    ) -> Self {
        Self {
            root: PaneNode::new_pane(
                columns,
                rows,
            ),
        }
    }

    pub fn terminal(&self) -> &Terminal {
        self.root
            .first_pane()
            .terminal()
    }

    pub fn terminal_mut(
        &mut self,
    ) -> &mut Terminal {
        self.root
            .first_pane_mut()
            .terminal_mut()
    }

    pub fn write(
        &mut self,
        text: &str,
    ) {
        self.root
            .first_pane_mut()
            .write(text);
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

    pub fn active_pane(
        &self,
    ) -> &Pane {
        self.root.first_pane()
    }

    pub fn active_pane_mut(
        &mut self,
    ) -> &mut Pane {
        self.root.first_pane_mut()
    }

    pub fn split_active(
        &mut self,
        direction: SplitDirection,
    ) {
        let columns = self.terminal().width();
        let rows = self.terminal().height();

        self.root.split(
            direction,
            columns,
            rows,
        );
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
}
