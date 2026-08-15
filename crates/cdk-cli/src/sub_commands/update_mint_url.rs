use anyhow::Result;
use cdk::mint_url::MintUrl;
use cdk::wallet::WalletRepository;
use clap::Args;

#[derive(Args)]
pub struct UpdateMintUrlSubCommand {
    /// Mint identifier: public key or current URL
    mint_id: String,
    /// New Mint Url
    new_mint_url: MintUrl,
}

pub async fn update_mint_url(
    wallet_repository: &WalletRepository,
    sub_command_args: &UpdateMintUrlSubCommand,
) -> Result<()> {
    let UpdateMintUrlSubCommand {
        mint_id,
        new_mint_url,
    } = sub_command_args;

    wallet_repository
        .update_mint_url(mint_id.as_str(), new_mint_url.clone())
        .await?;

    println!("Mint {mint_id} now at {new_mint_url}");

    Ok(())
}
