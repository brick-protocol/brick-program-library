use {
    anchor_lang::{
        prelude::*,
        solana_program::{ program_memory::sol_memcmp, pubkey::PUBKEY_BYTES },
    },
    anchor_spl::{
        token_interface::{ Mint, TokenInterface, TokenAccount },
        associated_token::AssociatedToken,
        token::{ transfer, Transfer },
        token_interface::{ CloseAccount, close_account },
    }
};

declare_id!("6NSfzFwHeuDCLzFwAo3yQ2KLLb9bThvkEVyeWChoAqBa");

#[program]
pub mod product_manager {
    use super::*;

    pub fn init_product(ctx: Context<InitProduct>, id: [u8; 16], price: u64) -> Result<()> {
        (*ctx.accounts.product).id = id;
        (*ctx.accounts.product).authority = ctx.accounts.signer.key();
        (*ctx.accounts.product).payment_mint = ctx.accounts.payment_mint.key();
        (*ctx.accounts.product).price = price;
        (*ctx.accounts.product).bump = *ctx.bumps.get("product").unwrap();

        Ok(())
    }

    pub fn pay(ctx: Context<Pay>, expire_time: i64) -> Result<()> {
        (*ctx.accounts.escrow).buyer = ctx.accounts.signer.key();
        (*ctx.accounts.escrow).seller = ctx.accounts.seller.key();
        (*ctx.accounts.escrow).expire_time = expire_time;
        (*ctx.accounts.escrow).vault_bump = *ctx.bumps.get("escrow_vault").unwrap();
        (*ctx.accounts.escrow).bump = *ctx.bumps.get("escrow").unwrap();

        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.transfer_vault.to_account_info(),
                    to: ctx.accounts.escrow_vault.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info(),
                },
            ),
            ctx.accounts.product.price,
        )?;

        Ok(())
    }

    pub fn accept(ctx: Context<Accept>) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        if now > ctx.accounts.escrow.expire_time {
            return Err(ErrorCode::TimeExpired.into());
        }

        let product_key = ctx.accounts.product.key();
        let signer_key = ctx.accounts.product.key();
    
        let escrow_seeds = [
            b"escrow".as_ref(),
            product_key.as_ref(),
            signer_key.as_ref(),
            &[ctx.accounts.escrow.bump],
        ];
    
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    to: ctx.accounts.transfer_vault.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                &[&escrow_seeds[..]]
            ),
            ctx.accounts.transfer_vault.amount
        )?;

        close_account( 
            CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            CloseAccount {
                account: ctx.accounts.escrow_vault.to_account_info(),
                destination: ctx.accounts.buyer.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            &[&escrow_seeds[..]],
        ))?;

        Ok(())
    }

    pub fn deny(ctx: Context<Deny>) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        if now > ctx.accounts.escrow.expire_time {
            return Err(ErrorCode::TimeExpired.into());
        }

        let product_key = ctx.accounts.product.key();
        let signer_key = ctx.accounts.product.key();
    
        let escrow_seeds = [
            b"escrow".as_ref(),
            product_key.as_ref(),
            signer_key.as_ref(),
            &[ctx.accounts.escrow.bump],
        ];
    
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    to: ctx.accounts.transfer_vault.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                &[&escrow_seeds[..]]
            ),
            ctx.accounts.transfer_vault.amount
        )?;

        close_account( 
            CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            CloseAccount {
                account: ctx.accounts.escrow_vault.to_account_info(),
                destination: ctx.accounts.buyer.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            &[&escrow_seeds[..]],
        ))?;

        Ok(())
    }

    pub fn recover_funds(ctx: Context<RecoverFunds>) -> Result<()> {
        let now = Clock::get().unwrap().unix_timestamp;
        if now < ctx.accounts.escrow.expire_time {
            return Err(ErrorCode::CannotRecoverYet.into());
        }

        let product_key = ctx.accounts.product.key();
        let signer_key = ctx.accounts.product.key();
    
        let escrow_seeds = [
            b"escrow".as_ref(),
            product_key.as_ref(),
            signer_key.as_ref(),
            &[ctx.accounts.escrow.bump],
        ];
    
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    to: ctx.accounts.transfer_vault.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info(),
                },
                &[&escrow_seeds[..]]
            ),
            ctx.accounts.transfer_vault.amount
        )?;

        close_account( 
            CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            CloseAccount {
                account: ctx.accounts.escrow_vault.to_account_info(),
                destination: ctx.accounts.signer.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            &[&escrow_seeds[..]],
        ))?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(id: [u8; 16])]
pub struct InitProduct<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        space = PRODUCT_SIZE,
        payer = signer,
        seeds = [
            b"product".as_ref(),
            signer.key().as_ref(),
            id.as_ref()
        ],
        bump
    )]
    pub product: Account<'info, Product>,
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Pay<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub seller: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"product".as_ref(),
            seller.key().as_ref(),
            product.id.as_ref()
        ],
        bump = product.bump,
        constraint = product.authority == seller.key()
            @ ErrorCode::IncorrectAuthority,
        constraint = product.payment_mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub product: Account<'info, Product>,
    #[account(
        init,
        payer = signer,
        space = ESCORW_SIZE,
        seeds = [
            b"escrow".as_ref(),
            product.key().as_ref(),
            signer.key().as_ref()
        ],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = signer,
        seeds = [
            b"escrow_vault".as_ref(),
            product.key().as_ref(),
            signer.key().as_ref()
        ],
        bump,
        token::mint = payment_mint,
        token::authority = escrow,
    )]
    pub escrow_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = transfer_vault.owner == signer.key()
            @ ErrorCode::IncorrectOwner,
        constraint = transfer_vault.mint == product.payment_mint
            @ ErrorCode::IncorrectMint,
    )]
    pub transfer_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct Accept<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub buyer: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"product".as_ref(),
            signer.key().as_ref(),
            product.id.as_ref()        
        ],
        bump = product.bump,
        constraint = signer.key() == product.authority 
            @ ErrorCode::IncorrectAuthority,
        constraint = product.payment_mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub product: Account<'info, Product>,
    #[account(
        mut,
        seeds = [
            b"escrow".as_ref(),
            product.key().as_ref(),
            buyer.key().as_ref()
        ],
        bump = escrow.bump,
        constraint = escrow.seller == signer.key()
            && escrow.buyer == buyer.key() 
            @ ErrorCode::IncorrectParticipant,
        close = buyer,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        seeds = [
            b"escrow_vault".as_ref(),
            product.key().as_ref(),
            buyer.key().as_ref()
        ],
        bump = escrow.vault_bump,
        constraint = escrow_vault.owner == escrow.key()
            @ ErrorCode::IncorrectOwner,
        constraint = escrow_vault.mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub escrow_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = transfer_vault.owner == signer.key()
            @ ErrorCode::IncorrectOwner,
        constraint = transfer_vault.mint == product.payment_mint
            @ ErrorCode::IncorrectMint,
    )]
    pub transfer_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct Deny<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub buyer: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"product".as_ref(),
            signer.key().as_ref(),
            product.id.as_ref()        
        ],
        bump = product.bump,
        constraint = signer.key() == product.authority 
            @ ErrorCode::IncorrectAuthority,
        constraint = product.payment_mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub product: Account<'info, Product>,
    #[account(
        mut,
        seeds = [
            b"escrow".as_ref(),
            product.key().as_ref(),
            buyer.key().as_ref()
        ],
        bump = escrow.bump,
        constraint = escrow.seller == signer.key()
            && escrow.buyer == buyer.key() 
            @ ErrorCode::IncorrectParticipant,
        close = buyer,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        seeds = [
            b"escrow_vault".as_ref(),
            product.key().as_ref(),
            buyer.key().as_ref()
        ],
        bump = escrow.vault_bump,
        constraint = escrow_vault.owner == escrow.key()
            @ ErrorCode::IncorrectOwner,
        constraint = escrow_vault.mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub escrow_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = transfer_vault.owner == buyer.key()
            @ ErrorCode::IncorrectOwner,
        constraint = transfer_vault.mint == product.payment_mint
            @ ErrorCode::IncorrectMint,
    )]
    pub transfer_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct RecoverFunds<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub seller: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [
            b"product".as_ref(),
            seller.key().as_ref(),
            product.id.as_ref()        
        ],
        bump = product.bump,
        constraint = seller.key() == product.authority 
            @ ErrorCode::IncorrectAuthority,
        constraint = product.payment_mint == payment_mint.key()
            @ ErrorCode::IncorrectMint
    )]
    pub product: Account<'info, Product>,
    #[account(
        mut,
        seeds = [
            b"escrow".as_ref(),
            product.key().as_ref(),
            signer.key().as_ref()
        ],
        bump = escrow.bump,
        constraint = escrow.seller == seller.key()
            && escrow.buyer == signer.key() 
            @ ErrorCode::IncorrectParticipant,
        close = signer,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        seeds = [
            b"escrow_vault".as_ref(),
            product.key().as_ref(),
            signer.key().as_ref()
        ],
        bump = escrow.vault_bump,
        constraint = escrow_vault.owner == escrow.key()
            @ ErrorCode::IncorrectOwner,
        constraint = escrow_vault.mint == product.payment_mint
            @ ErrorCode::IncorrectMint,
    )]
    pub escrow_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = transfer_vault.owner == signer.key()
            @ ErrorCode::IncorrectOwner,
        constraint = transfer_vault.mint == product.payment_mint
            @ ErrorCode::IncorrectMint,
    )]
    pub transfer_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

#[account]
pub struct Product {
    pub id: [u8; 16],
    pub authority: Pubkey,
    pub payment_mint: Pubkey,
    pub price: u64,
    pub bump: u8
}

pub const PRODUCT_SIZE: usize = 8 + 16 + 32 + 32 + 8 + 1;

#[account]
pub struct Escrow {
    /// depending on the blocktime the authority is the buyer or the seller
    /// seller can accept or deny propossal before expire time
    /// buyer can recover funds after expire time
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub expire_time: i64,
    pub vault_bump: u8,
    pub bump: u8,
}

pub const ESCORW_SIZE: usize = 8 + 32 + 32 + 8 + 1 + 1;

#[error_code]
pub enum ErrorCode {
    #[msg("Wrong authority")]
    IncorrectAuthority,
    #[msg("Wrong owner on a token account")]
    IncorrectOwner,
    #[msg("Wrong mint on a token account")]
    IncorrectMint,
    #[msg("Wrong participant of the escrow")]
    IncorrectParticipant,
    #[msg("Your time to accept or deny propossal has expired")]
    TimeExpired,
    #[msg("Payment recovery is not allowed at this time")]
    CannotRecoverYet,
}