use std::any::Any;
use std::sync::Arc;
use std::sync::RwLock;

use pumpkin_data::{item_stack::ItemStack, screen::WindowType};
use pumpkin_inventory::player::player_inventory::PlayerInventory;
use pumpkin_inventory::screen_handler::{InventoryPlayer, ScreenHandler, ScreenHandlerBehaviour};
use pumpkin_inventory::slot::NormalSlot;
use pumpkin_inventory::{Clearable, Inventory};
use pumpkin_util::text::TextComponent;

pub struct PluginGui {
    pub window_type: WindowType,
    pub title: TextComponent,
    pub inventory: Arc<PluginInventory>,
    pub allow_grab_items: bool,
    pub allow_put_items: bool,
}

pub struct PluginInventory {
    pub slots: RwLock<Vec<ItemStack>>,
}

impl PluginInventory {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self {
            slots: RwLock::new(vec![ItemStack::EMPTY.clone(); size]),
        }
    }
}

impl Clearable for PluginInventory {
    fn clear(&self) {
        let mut slots = self
            .slots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slots.fill_with(|| ItemStack::EMPTY.clone());
    }
}

impl Inventory for PluginInventory {
    fn size(&self) -> usize {
        self.slots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    fn is_empty(&self) -> bool {
        let slots = self
            .slots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slots.iter().all(ItemStack::is_empty)
    }

    fn get_stack(&self, slot: usize) -> ItemStack {
        let slots = self
            .slots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        slots
            .get(slot)
            .cloned()
            .unwrap_or_else(|| ItemStack::EMPTY.clone())
    }

    fn remove_stack(&self, slot: usize) -> ItemStack {
        let mut slots = self
            .slots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < slots.len() {
            std::mem::replace(&mut slots[slot], ItemStack::EMPTY.clone())
        } else {
            ItemStack::EMPTY.clone()
        }
    }

    fn remove_stack_specific(&self, slot: usize, amount: u8) -> ItemStack {
        let mut slots = self
            .slots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < slots.len() && !slots[slot].is_empty() && amount > 0 {
            slots[slot].split(amount)
        } else {
            ItemStack::EMPTY.clone()
        }
    }

    fn set_stack(&self, slot: usize, stack: ItemStack) {
        let mut slots = self
            .slots
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if slot < slots.len() {
            slots[slot] = stack;
        }
    }

    fn on_open(&self) {}

    fn on_close(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub struct PluginScreenHandler {
    pub inventory: Arc<PluginInventory>,
    behaviour: ScreenHandlerBehaviour,
}

impl PluginScreenHandler {
    #[must_use]
    pub fn new(
        sync_id: u8,
        window_type: WindowType,
        inventory: &Arc<PluginInventory>,
        player_inventory: &Arc<PlayerInventory>,
        allow_grab_items: bool,
        allow_put_items: bool,
    ) -> Self {
        let mut behaviour = ScreenHandlerBehaviour::new(sync_id, Some(window_type));
        behaviour.allow_grab_items = allow_grab_items;
        behaviour.allow_put_items = allow_put_items;
        behaviour.container_slots = inventory.size();

        let mut handler = Self {
            inventory: inventory.clone(),
            behaviour,
        };

        for i in 0..inventory.size() {
            handler.add_slot(Arc::new(NormalSlot::new(inventory.clone(), i)));
        }

        // The client lays the player's inventory out right after the container slots, so it only
        // lines up when the container has the window's vanilla slot count.
        if has_player_slots(window_type, inventory.size()) {
            let player_inventory: Arc<dyn Inventory> = player_inventory.clone();
            handler.add_player_slots(&player_inventory);
        }

        handler
    }
}

/// Whether a plugin GUI of this window type and container size can show the player's inventory.
const fn has_player_slots(window_type: WindowType, container_size: usize) -> bool {
    matches!(
        (window_type, container_size),
        (WindowType::Generic9x1 | WindowType::Generic3x3, 9)
            | (WindowType::Generic9x2, 18)
            | (WindowType::Generic9x3 | WindowType::ShulkerBox, 27)
            | (WindowType::Generic9x4, 36)
            | (WindowType::Generic9x5, 45)
            | (WindowType::Generic9x6, 54)
            | (WindowType::Hopper, 5)
    )
}

impl ScreenHandler for PluginScreenHandler {
    fn on_closed(&mut self, player: &dyn InventoryPlayer) {
        self.default_on_closed(player);
        self.inventory.on_close();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_behaviour(&self) -> &ScreenHandlerBehaviour {
        &self.behaviour
    }

    fn get_behaviour_mut(&mut self) -> &mut ScreenHandlerBehaviour {
        &mut self.behaviour
    }

    fn quick_move(&mut self, _player: &dyn InventoryPlayer, slot_index: i32) -> ItemStack {
        let container_slots = self.behaviour.container_slots as i32;
        let total_slots = self.behaviour.slots.len() as i32;
        if total_slots == container_slots {
            // No player slots to move to or from.
            return ItemStack::EMPTY.clone();
        }

        let mut stack_left = ItemStack::EMPTY.clone();
        let slot = self.behaviour.slots[slot_index as usize].clone();

        if slot.has_stack() {
            let mut slot_stack = slot.get_stack();
            stack_left = slot_stack.clone();

            if slot_index < container_slots {
                // From the container to the player
                if !self.behaviour.allow_grab_items
                    || !self.insert_item(&mut slot_stack, container_slots, total_slots, true)
                {
                    return ItemStack::EMPTY.clone();
                }
            } else if !self.behaviour.allow_put_items
                || !self.insert_item(&mut slot_stack, 0, container_slots, false)
            {
                // From the player to the container
                return ItemStack::EMPTY.clone();
            }

            if slot_stack.is_empty() {
                slot.set_stack(ItemStack::EMPTY.clone());
            } else {
                slot.set_stack(slot_stack);
            }
        }

        stack_left
    }
}
