#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_confirm_teleport(&self, player: &Player, confirm_teleport: &SConfirmTeleport) {
        enum TeleportResult {
            Success,
            Outdated,
            NotTeleporting,
        }

        let result = {
            let mut awaiting_teleport = player
                .awaiting_teleport
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // Like vanilla's handleAcceptTeleportPacket, only the latest teleport counts: the
            // client still confirms one that a newer teleport replaced before it arrived.
            if confirm_teleport.teleport_id.0 != player.teleport_id_count.load(Ordering::Relaxed) {
                TeleportResult::Outdated
            } else if let Some((_, position)) = awaiting_teleport.take() {
                // We should set the position now to what we requested in the teleport packet.
                // This may fix issues when the client sends the position while being teleported.
                player.get_entity().set_pos(position);
                TeleportResult::Success
            } else {
                TeleportResult::NotTeleporting
            }
        };

        match result {
            TeleportResult::Success | TeleportResult::Outdated => {}
            TeleportResult::NotTeleporting => {
                self.try_kick(&TextComponent::text(
                    "Send Teleport confirm, but we did not teleport",
                ));
            }
        }
    }
}
