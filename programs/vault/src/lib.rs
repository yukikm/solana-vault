// lint detects unexpected conditional compilation conditions
#![allow(unexpected_cfgs)]
// lint detects deprecated items
#![allow(deprecated)]
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

declare_id!("UCrARA7PhDE2jwhXLj8jUUptRRjXZjneUViRFRYCJt1");

#[program]
pub mod vault {
    // super is parent. importing all items from the parent module
    use super::*;
    // Context is used to pass accounts and bumps to the instruction functions
    // if you want to access accounts and bumps, you need to use Context
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        // ? is used to handle Result types
        // it will return an error if the operation fails
        // if it succeeds, it will return Ok(())
        // match ctx.accounts.initialize(&ctx.bumps) {
        //     Ok(_) => {}
        //     Err(e) => return Err(e),
        // }
        ctx.accounts.initialize(&ctx.bumps)?;
        // ctx.bumps is a struct that contains the bump values for the accounts
        Ok(())
    }

    pub fn deposit(ctx: Context<Payment>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Payment>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        ctx.accounts.close()
    }
}

// trait is used to define common functionality for structs
// derive is import macro that automatically implements the trait for the struct
// this example uses Accounts trait to define the accounts that are used in the instruction
#[derive(Accounts)]
// info is a lifetime parameter that is used to define the lifetime of the accounts
pub struct Initialize<'info> {
    // Signer is used to define the account that is signing the transaction
    // mut is used to define that the account is mutable
    // this means that the account can be modified by the instruction
    // if you use mut, you need use existing account
    #[account(mut)]
    pub user: Signer<'info>,
    // if you want to use an non-existing account, you need to use init and create a new account
    #[account(
        init,
        payer = user,
        space = VaultState::INIT_SPACE,
        // Checks that given account is a PDA derived from the currently executing program, the seeds, and if provided, the bump seed.
        seeds = [b"state", user.key().as_ref()],
        bump
    )]
    pub vault_state: Account<'info, VaultState>, // PDA
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump
    )]
    // Type validating that the account is of type SystemAccount, which is a wrapper around the system program account.
    // You can use SystemAccount<'info> in your instruction context, especially for accounts like program-derived addresses (PDAs) intended to hold SOL. For example, to create a PDA with no data, you can use:
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

// https://www.anchor-lang.com/docs/references/account-types
// https://www.anchor-lang.com/docs/references/account-constraints

// // without lifetime annotations, the compiler cannot determine how long the reference is valid
// struct Bad {
//     data: &str,  // compiler cannot determine the lifetime. This will cause an error
// }

// // with lifetime annotations, the compiler can determine how long the reference is valid
// struct Good<'a> {
//     data: &'a str,  // 'a is a lifetime parameter
// }

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        // Ensure the vault account is rent-exempt
        // Rent is a system that ensures that accounts have enough SOL to be kept alive
        // calculate the minimum balance required for the vault account
        let rent_exempt = Rent::get()?.minimum_balance(self.vault.to_account_info().data_len());
        //
        let cpi_program = self.system_program.to_account_info();
        // Transfer is a CPI (Cross-Program Invocation) that allows transferring SOL from one account to another
        // Transfer struct
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };

        // CpiContext is used to create a context for the CPI
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(cpi_ctx, rent_exempt)?;

        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Payment<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Payment<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();
        let cpi_account = Transfer {
            from: self.user.to_account_info(),
            to: self.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_account);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();
        let cpi_account = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        // PDA signing is required for the transfer
        // because the vault account is a PDA (Program Derived Address)
        // and it needs to be signed by the PDA's seeds
        // to ensure that the transfer is valid
        // this is done by using CpiContext::new_with_signer
        // which allows you to specify the seeds that will be used to sign the transaction

        // & is used to create a reference to the seeds
        let seeds = &[
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];

        // Why use a reference?
        // Because the seeds are used to derive the PDA, and we need to pass a reference
        // let seeds = [b"vault", ...];  // move ownership of the array
        // // create a reference to the array
        // let seeds = &[b"vault", ...]; // borrow the array

        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_account, signer_seeds);
        transfer(cpi_ctx, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", vault_state.key().as_ref()],
        bump = vault_state.vault_bump,)]
    pub vault: SystemAccount<'info>,
    #[account(
        mut,
        seeds = [b"state", user.key().as_ref()],
        bump = vault_state.state_bump,
        close = user,
    )]
    pub vault_state: Account<'info, VaultState>,
    pub system_program: Program<'info, System>,
}

impl<'info> Close<'info> {
    pub fn close(&mut self) -> Result<()> {
        let cpi_program = self.system_program.to_account_info();
        let cpi_account = Transfer {
            from: self.vault.to_account_info(),
            to: self.user.to_account_info(),
        };
        let pda_signing_seeds = [
            b"vault",
            self.vault_state.to_account_info().key.as_ref(),
            &[self.vault_state.vault_bump],
        ];
        let seeds = &[&pda_signing_seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_account, seeds);
        transfer(cpi_ctx, self.vault.lamports())?;
        Ok(())
    }
}

#[account]
pub struct VaultState {
    pub vault_bump: u8,
    pub state_bump: u8,
}

impl Space for VaultState {
    const INIT_SPACE: usize = 8 + 1 * 2; // 8 bytes for discriminator + 1 byte for vault_bump + 1 byte for state_bump
}
