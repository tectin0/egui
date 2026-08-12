use ahash::HashMap;

use crate::{Id, IdMap, LayerId, Rect, Sense, WidgetInfo};

/// Used to store each widget's [Id], [Rect] and [Sense] each frame.
///
/// Used to check which widget gets input when a user clicks somewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetRect {
    /// The globally unique widget id.
    ///
    /// For interactive widgets, this better be globally unique.
    /// If not there will be weird bugs,
    /// and also big red warning test on the screen in debug builds
    /// (see [`crate::Options::warn_on_id_clash`]).
    ///
    /// You can ensure globally unique ids using [`crate::Ui::push_id`].
    pub id: Id,

    /// The [`Id`] of the parent [`crate::Ui`] that hosts this widget.
    ///
    /// Used by debug checks to distinguish true id-instability from
    /// cascading id shifts caused by a parent Ui's auto-id changing.
    pub parent_id: Id,

    /// What layer the widget is on.
    pub layer_id: LayerId,

    /// The full widget rectangle, in local layer coordinates.
    pub rect: Rect,

    /// Where the widget is, in local layer coordinates.
    ///
    /// This is after clipping with the parent ui clip rect.
    pub interact_rect: Rect,

    /// How the widget responds to interaction.
    ///
    /// Note: if [`Self::enabled`] is `false`, then
    /// the widget _effectively_ doesn't sense anything,
    /// but can still have the same `Sense`.
    /// This is because the sense informs the styling of the widget,
    /// but we don't want to change the style when a widget is disabled
    /// (that is handled by the `Painter` directly).
    pub sense: Sense,

    /// Is the widget enabled?
    pub enabled: bool,

    /// Was the widget's data changed this frame?
    ///
    /// Set by [`Response::mark_changed`](crate::Response::mark_changed).
    pub changed: bool,

    /// Was any descendant widget's data changed this frame?
    ///
    /// Propagated upward through the parent chain when a descendant calls
    /// [`Response::mark_changed`](crate::Response::mark_changed).
    pub child_changed: bool,

    /// The stable [`Id`] of this widget, if it is a [`Ui`](crate::Ui).
    ///
    /// For non-Ui widgets this is `None`. For Uis, this is `Ui::id()` — the stable id
    /// that children see as their `parent_id`. This differs from `WidgetRect::id`
    /// (which is `Ui::unique_id()`) for child Uis.
    pub stable_id: Option<Id>,
}

impl WidgetRect {
    pub fn transform(self, transform: emath::TSTransform) -> Self {
        let Self {
            id,
            parent_id,
            layer_id,
            rect,
            interact_rect,
            sense,
            enabled,
            changed,
            child_changed,
            stable_id,
        } = self;
        Self {
            id,
            parent_id,
            layer_id,
            rect: transform * rect,
            interact_rect: transform * interact_rect,
            sense,
            enabled,
            changed,
            child_changed,
            stable_id,
        }
    }
}

/// How to handle multiple calls to [`crate::Response::interact`] and [`crate::Ui::interact_opt`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InteractOptions {
    /// If we call interact on the same widget multiple times,
    /// should we move it to the top on subsequent calls?
    pub move_to_top: bool,
}

#[expect(clippy::derivable_impls)] // Nice to be explicit
impl Default for InteractOptions {
    fn default() -> Self {
        Self { move_to_top: false }
    }
}

/// Stores the [`WidgetRect`]s of all widgets generated during a single egui update/frame.
///
/// All [`crate::Ui`]s have a [`WidgetRect`]. It is created in [`crate::Ui::new`] with [`Rect::NOTHING`]
/// and updated with the correct [`Rect`] when the [`crate::Ui`] is dropped.
#[derive(Default, Clone)]
pub struct WidgetRects {
    /// All widgets, in painting order.
    by_layer: HashMap<LayerId, Vec<WidgetRect>>,

    /// All widgets, by id, and their order in their respective layer
    by_id: IdMap<(usize, WidgetRect)>,

    /// Maps a Ui's stable id to its unique id, for Uis where they differ.
    stable_to_id: IdMap<Id>,

    /// Info about some widgets.
    ///
    /// Only filled in if the widget is interacted with,
    /// or if this is a debug build.
    infos: IdMap<WidgetInfo>,
}

impl PartialEq for WidgetRects {
    fn eq(&self, other: &Self) -> bool {
        self.by_layer == other.by_layer
    }
}

impl WidgetRects {
    /// All known layers with widgets.
    pub fn layer_ids(&self) -> impl ExactSizeIterator<Item = LayerId> + '_ {
        self.by_layer.keys().copied()
    }

    pub fn layers(&self) -> impl Iterator<Item = (&LayerId, &[WidgetRect])> + '_ {
        self.by_layer
            .iter()
            .map(|(layer_id, rects)| (layer_id, &rects[..]))
    }

    #[inline]
    pub fn get(&self, id: Id) -> Option<&WidgetRect> {
        self.by_id.get(&id).map(|(_, w)| w)
    }

    /// In which layer, and in which order in that layer?
    pub fn order(&self, id: Id) -> Option<(LayerId, usize)> {
        self.by_id.get(&id).map(|(idx, w)| (w.layer_id, *idx))
    }

    #[inline]
    pub fn contains(&self, id: Id) -> bool {
        self.by_id.contains_key(&id)
    }

    /// All widgets in this layer, sorted back-to-front.
    #[inline]
    pub fn get_layer(&self, layer_id: LayerId) -> impl Iterator<Item = &WidgetRect> + '_ {
        self.by_layer.get(&layer_id).into_iter().flatten()
    }

    /// Clear the contents while retaining allocated memory.
    pub fn clear(&mut self) {
        let Self {
            by_layer,
            by_id,
            stable_to_id,
            infos,
        } = self;

        #[expect(clippy::iter_over_hash_type)]
        for rects in by_layer.values_mut() {
            rects.clear();
        }

        by_id.clear();
        stable_to_id.clear();

        infos.clear();
    }

    /// Insert the given widget rect in the given layer.
    pub fn insert(&mut self, layer_id: LayerId, widget_rect: WidgetRect, options: InteractOptions) {
        let Self {
            by_layer,
            by_id,
            stable_to_id,
            infos: _,
        } = self;

        let InteractOptions { move_to_top } = options;

        let mut shift_layer_index_after = None;

        let layer_widgets = by_layer.entry(layer_id).or_default();

        match by_id.entry(widget_rect.id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                // A new widget
                let idx_in_layer = layer_widgets.len();
                if let Some(s) = widget_rect.stable_id {
                    stable_to_id.insert(s, widget_rect.id);
                }
                entry.insert((idx_in_layer, widget_rect));
                layer_widgets.push(widget_rect);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                // This is a known widget, but we might need to update it!
                // e.g. calling `response.interact(…)` to add more interaction.
                let (idx_in_layer, existing) = entry.get_mut();

                // Update it:
                existing.rect = widget_rect.rect; // last wins
                existing.interact_rect = widget_rect.interact_rect; // last wins
                existing.sense |= widget_rect.sense;
                existing.enabled |= widget_rect.enabled;

                if existing.layer_id == widget_rect.layer_id {
                    if move_to_top {
                        layer_widgets.remove(*idx_in_layer);
                        shift_layer_index_after = Some(*idx_in_layer);
                        *idx_in_layer = layer_widgets.len();
                        layer_widgets.push(*existing);
                    } else {
                        layer_widgets[*idx_in_layer] = *existing;
                    }
                } else if cfg!(debug_assertions) {
                    panic!(
                        "DEBUG ASSERT: Widget {:?} changed layer_id during the frame from {:?} to {:?}",
                        widget_rect.id, existing.layer_id, widget_rect.layer_id
                    );
                }
            }
        }

        if let Some(shift_start) = shift_layer_index_after {
            #[expect(clippy::needless_range_loop)]
            for i in shift_start..layer_widgets.len() {
                let w = &layer_widgets[i];
                if let Some((idx_in_by_id, _)) = by_id.get_mut(&w.id) {
                    *idx_in_by_id = i;
                }
            }
        }
    }

    pub fn set_info(&mut self, id: Id, info: WidgetInfo) {
        self.infos.insert(id, info);
    }

    pub fn info(&self, id: Id) -> Option<&WidgetInfo> {
        self.infos.get(&id)
    }

    /// Mark a widget as changed by its [`Id`], and propagate
    /// [`WidgetRect::child_changed`] up the parent chain.
    pub fn mark_changed(&mut self, id: Id) {
        // Set changed on the widget itself
        let parent_id = match self.by_id.get_mut(&id) {
            Some((_, wr)) => {
                wr.changed = true;
                wr.parent_id
            }
            None => return,
        };

        // Walk up the parent chain, setting child_changed on each ancestor.
        // `parent_id` stores the parent's stable id; `stable_to_id` maps that to `unique_id`
        // so we can look up the parent's WidgetRect in `by_id`.
        let mut current = parent_id;

        while let Some(unique_id) = self.stable_to_id.get(&current).copied() {
            match self.by_id.get_mut(&unique_id) {
                Some((_, wr)) if !wr.child_changed => {
                    wr.child_changed = true;
                    current = wr.parent_id;
                }
                _ => break,
            }
        }
    }

    /// Returns `true` if any descendant widget of `parent_id` has its data changed this frame.
    ///
    /// This is an O(1) check — the [`WidgetRect::child_changed`] flag is propagated upward
    /// when [`WidgetRect::changed`] is set via [`Response::mark_changed`](crate::Response::mark_changed).
    pub fn any_child_changed(&self, parent_id: Id) -> bool {
        self.by_id
            .get(&parent_id)
            .is_some_and(|(_, wr)| wr.child_changed)
    }
}
