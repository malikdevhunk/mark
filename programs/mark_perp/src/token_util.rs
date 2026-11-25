use anchor_lang::prelude::*;
use anchor_lang::Key;
use anchor_spl::token::{self, Transfer};

use crate::errors::MarkError;

pub fn token_balance(data: &[u8]) -> Result<u64> {
    if data.len() < 72 {
        return err!(MarkError::Math);
    }
    let bytes: [u8; 8] = data[64..72].try_into().map_err(|_| MarkError::Math)?;
    Ok(u64::from_le_bytes(bytes))
}

pub fn transfer_as_exchange<'info>(
    token_program: AccountInfo<'info>,
    from: AccountInfo<'info>,
    to: AccountInfo<'info>,
    authority: AccountInfo<'info>,
    bump: u8,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[b"exchange", &bump_seed];
    token::transfer(
        CpiContext::new_with_signer(
            token_program.key(),
            Transfer {
                from,
                to,
                authority,
            },
            &[seeds],
        ),
        amount,
    )
}
