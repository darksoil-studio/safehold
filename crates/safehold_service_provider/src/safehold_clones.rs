use anyhow::anyhow;
use holochain::prelude::{
    AgentPubKey, CloneCellId, CreateCloneCellPayload, DeleteCloneCellPayload,
    DisableCloneCellPayload, DnaModifiersOpt, RoleName, YamlProperties,
};
use holochain_client::{
    AdminWebsocket, AppWebsocket, CellInfo, ClonedCell, ExternIO, Timestamp, ZomeCallTarget,
};
use roles_types::Properties;
use safehold_types::MessageWithProvenance;

pub async fn reconcile_safehold_clones(
    admin_ws: &AdminWebsocket,
    app_ws: &AppWebsocket,
    progenitors: Vec<AgentPubKey>,
) -> anyhow::Result<()> {
    let current_network_seed = get_current_time_epoch();

    let Some(app_info) = app_ws.app_info().await? else {
        return Err(anyhow!("app_info() returned None"));
    };

    let safehold_cells = app_info
        .cell_info
        .get("safehold")
        .cloned()
        .unwrap_or_default();

    let existing_cloned_cells: Vec<ClonedCell> = safehold_cells
        .iter()
        .filter_map(|c| match c {
            CellInfo::Cloned(cloned) => Some(cloned.clone()),
            _ => None,
        })
        .collect();

    let already_exists = existing_cloned_cells
        .iter()
        .find(|c| c.enabled && c.dna_modifiers.network_seed.eq(&current_network_seed))
        .is_some();

    if !already_exists {
        log::info!("New epoch time reached: deleting the current safehold cell if it exists and creating a new one.");

        let roles_properties = Properties {
            progenitors: progenitors.clone().into_iter().map(|p| p.into()).collect(),
        };
        let value = serde_yaml::to_value(roles_properties).unwrap();
        let properties_bytes = YamlProperties::new(value);

        let cloned_cell = app_ws
            .create_clone_cell(CreateCloneCellPayload {
                role_name: RoleName::from("safehold"),
                modifiers: DnaModifiersOpt {
                    properties: Some(properties_bytes.clone()),
                    network_seed: Some(current_network_seed.clone()),
                },
                membrane_proof: None,
                name: None,
            })
            .await?;

        app_ws
            .call_zome(
                ZomeCallTarget::RoleName("proxy".into()),
                "proxy".into(),
                "create_proxied_dna".into(),
                ExternIO::encode(cloned_cell.cell_id.dna_hash().clone())?,
            )
            .await?;

        let previous_cell = existing_cloned_cells
            .iter()
            .filter(|c| c.enabled)
            .max_by_key(|c| c.clone_id.as_clone_index());
        if let Some(previous_cell) = previous_cell {
            let messages: Vec<MessageWithProvenance> = app_ws
                .call_zome(
                    ZomeCallTarget::CellId(previous_cell.cell_id.clone()),
                    "safehold".into(),
                    "export_undeleted_messages".into(),
                    ExternIO::encode(())?,
                )
                .await?
                .decode()?;

            log::info!(
                "Migrating {} messages from the old cell to the new one.",
                messages.len()
            );

            let _r: () = app_ws
                .call_zome(
                    ZomeCallTarget::CellId(cloned_cell.cell_id.clone()),
                    "safehold".into(),
                    "create_messages".into(),
                    ExternIO::encode(messages)?,
                )
                .await?
                .decode()?;
        }

        log::info!(
            "Successfully advanced safehold clone epoch, new clone: {}",
            cloned_cell.clone_id
        );
    }

    // Clean up remaining clones
    // Without this, whenever there is an error disabling or deleting the clone cell,
    // dangling clones will persist and never get cleaned up

    let dangling_cells: Vec<ClonedCell> = existing_cloned_cells
        .into_iter()
        .filter(|c| c.enabled && c.dna_modifiers.network_seed.ne(&current_network_seed))
        .collect();

    for dangling_cell in dangling_cells {
        log::info!(
            "Deleting the safehold clone cell for a previous epoch: {}.",
            dangling_cell.clone_id
        );
        app_ws
            .disable_clone_cell(DisableCloneCellPayload {
                clone_cell_id: CloneCellId::CloneId(dangling_cell.clone_id.clone()),
            })
            .await?;

        admin_ws
            .delete_clone_cell(DeleteCloneCellPayload {
                app_id: app_info.installed_app_id.clone(),
                clone_cell_id: CloneCellId::CloneId(dangling_cell.clone_id.clone()),
            })
            .await?;
    }

    Ok(())
}

const TIME_EPOCH_MINUTES: i64 = 10; // Every 10 minutes go over to another DHT

pub fn get_current_time_epoch() -> String {
    let timestamp = Timestamp::now();
    let minutes = timestamp.as_millis() / 1000 / 60;

    let epoch = minutes / TIME_EPOCH_MINUTES;

    format!("{epoch}")
}
