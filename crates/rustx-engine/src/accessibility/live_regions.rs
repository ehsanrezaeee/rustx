use accesskit::{Live, Node, NodeId, Role, TreeUpdate};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

#[derive(Default)]
pub(super) struct LiveRegions {
    messages: HashMap<NodeId, Message>,
}

struct Message {
    text: String,
    priority: Live,
    depth: usize,
    suspended: bool,
    muted: bool,
}

fn direct_label(node: &Node) -> Option<&str> {
    if node.role() == Role::Label {
        node.value()
    } else {
        node.label()
    }
}

impl LiveRegions {
    pub(super) fn update(&mut self, update: &mut TreeUpdate, depth: usize) {
        if self.messages.is_empty()
            && !update
                .nodes
                .iter()
                .any(|(_, node)| matches!(node.live(), Some(Live::Polite | Live::Assertive)))
        {
            return;
        }

        let index: HashMap<_, _> = update
            .nodes
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        let mut current = HashSet::new();
        let mut settings = Vec::new();
        let mut pending = vec![(super::ROOT, Live::Off)];
        while let Some((id, inherited)) = pending.pop() {
            let Some(&i) = index.get(&id) else { continue };
            let node = &update.nodes[i].1;
            let live = node.live().unwrap_or(inherited);
            pending.extend(node.children().iter().map(|id| (*id, live)));
            if live == Live::Off {
                continue;
            }

            let text = direct_label(node).map(Cow::Borrowed).or_else(|| {
                let labels = node
                    .labelled_by()
                    .iter()
                    .filter_map(|id| index.get(id))
                    .filter_map(|&i| direct_label(&update.nodes[i].1))
                    .collect::<Vec<_>>();
                (!labels.is_empty()).then(|| Cow::Owned(labels.join(" ")))
            });
            let mut effective = live;
            if let Some(text) = text {
                current.insert(id);
                let message = self.messages.entry(id).or_insert_with(|| Message {
                    text: text.as_ref().to_owned(),
                    priority: live,
                    depth,
                    suspended: false,
                    muted: false,
                });
                // AccessKit treats restored nodes as new announcements. Keep
                // an unchanged message quiet until its content changes.
                let unchanged = message.text == text.as_ref();
                message.muted =
                    unchanged && message.priority == live && (message.suspended || message.muted);
                message.suspended = false;
                message.depth = depth;
                if !unchanged {
                    message.text = text.into_owned();
                }
                message.priority = live;
                if message.muted {
                    effective = Live::Off;
                }
            }
            // Preserve the inherited priority for descendants of a muted node.
            settings.push((i, effective));
        }
        self.messages.retain(|id, message| {
            if current.contains(id) {
                true
            } else {
                message.suspended = depth > message.depth && !index.contains_key(id);
                message.suspended
            }
        });
        for (i, live) in settings {
            update.nodes[i].1.set_live(live);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use accesskit::TreeId;

    fn update(nodes: Vec<(NodeId, Node)>) -> TreeUpdate {
        let mut window = Node::new(Role::Window);
        window.set_children(nodes.iter().map(|(id, _)| *id).collect::<Vec<_>>());
        let mut nodes = nodes;
        nodes.push((super::super::ROOT, window));
        TreeUpdate {
            nodes,
            tree: None,
            tree_id: TreeId::ROOT,
            focus: super::super::ROOT,
        }
    }

    fn status(text: &str) -> (NodeId, Node) {
        let mut status = Node::new(Role::Status);
        status.set_label(text);
        status.set_live(Live::Polite);
        (NodeId(1), status)
    }

    #[test]
    fn restoring_a_modal_scope_only_announces_new_feedback() {
        let mut regions = LiveRegions::default();
        let mut before = update(vec![status("Review the form.")]);
        regions.update(&mut before, 0);
        assert_eq!(before.nodes[0].1.live(), Some(Live::Polite));

        regions.update(&mut update(vec![]), 1);
        regions.update(&mut update(vec![]), 2);
        regions.update(&mut update(vec![]), 1);
        for _ in 0..2 {
            let mut restored = update(vec![status("Review the form.")]);
            regions.update(&mut restored, 0);
            assert_eq!(restored.nodes[0].1.live(), Some(Live::Off));
            assert_eq!(restored.nodes[0].1.label(), Some("Review the form."));
        }

        regions.update(&mut update(vec![]), 1);
        let mut changed = update(vec![status("Profile submitted.")]);
        regions.update(&mut changed, 0);
        assert_eq!(changed.nodes[0].1.live(), Some(Live::Polite));

        regions.update(&mut update(vec![]), 0);
        let mut reinserted = update(vec![status("Profile submitted.")]);
        regions.update(&mut reinserted, 0);
        assert_eq!(reinserted.nodes[0].1.live(), Some(Live::Polite));
    }

    #[test]
    fn changed_child_keeps_its_priority_when_restored_parent_is_quiet() {
        let with_child = |text: &str| {
            let mut tree = update(vec![status("Status")]);
            let id = NodeId(2);
            tree.nodes[0].1.set_children(vec![id]);
            let mut child = Node::new(Role::Label);
            child.set_value(text);
            tree.nodes.push((id, child));
            tree
        };
        let mut regions = LiveRegions::default();
        regions.update(&mut with_child("Before"), 0);
        regions.update(&mut update(vec![]), 1);
        let mut restored = with_child("After");
        regions.update(&mut restored, 0);
        assert_eq!(restored.nodes[0].1.live(), Some(Live::Off));
        assert_eq!(restored.nodes[2].1.live(), Some(Live::Polite));
    }
}
