//! Lifecycle of an odds perp: deposit, open, funding, close, liquidation.
//! `anchor build` must have produced `target/deploy/mark_perp.so`.

use {
    litesvm::LiteSVM,
    sha2::{Digest, Sha256},
    solana_address::{address, Address},
    solana_clock::Clock,
    solana_instruction::{account_meta::AccountMeta, Instruction},
    solana_keypair::Keypair,
    solana_message::Message,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const ID: Address = address!("EwCuCZyudnEjbHZ6wFYZtZBnpJaGEYsX2X8VJiMh5MrG");
const TOKEN: Address = address!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
const ATA: Address = address!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const SYSTEM: Address = address!("11111111111111111111111111111111");

const SIZE: u64 = 1_000_000;
const NOW: i64 = 1_800_000_000;

fn sighash(name: &str) -> [u8; 8] {
    let hash = Sha256::digest(format!("global:{name}"));
    hash[..8].try_into().unwrap()
}

fn ix(name: &str, body: &[u8], accounts: Vec<AccountMeta>) -> Instruction {
    let mut data = sighash(name).to_vec();
    data.extend_from_slice(body);
    Instruction {
        program_id: ID,
        accounts,
        data,
    }
}

fn wr(pubkey: Address, signer: bool) -> AccountMeta {
    AccountMeta {
        pubkey,
        is_signer: signer,
        is_writable: true,
    }
}

fn ro(pubkey: Address) -> AccountMeta {
    AccountMeta {
        pubkey,
        is_signer: false,
        is_writable: false,
    }
}

fn pda(seeds: &[&[u8]]) -> Address {
    Address::find_program_address(seeds, &ID).0
}

fn ata_of(owner: &Address, mint: &Address) -> Address {
    Address::find_program_address(&[owner.as_ref(), TOKEN.as_ref(), mint.as_ref()], &ATA).0
}

fn send(svm: &mut LiteSVM, payer: &Keypair, instruction: Instruction, extra: &[&Keypair]) {
    let mut signers: Vec<&Keypair> = vec![payer];
    signers.extend_from_slice(extra);
    let msg = Message::new(&[instruction], Some(&payer.pubkey()));
    let tx = Transaction::new(&signers, msg, svm.latest_blockhash());
    match svm.send_transaction(tx) {
        Ok(_) => {}
        Err(err) => panic!("transaction failed:\n{err:#?}"),
    }
}

fn send_err(svm: &mut LiteSVM, payer: &Keypair, instruction: Instruction) -> String {
    let msg = Message::new(&[instruction], Some(&payer.pubkey()));
    let tx = Transaction::new(&[payer], msg, svm.latest_blockhash());
    match svm.send_transaction(tx) {
        Ok(ok) => panic!("expected failure, got {ok:#?}"),
        Err(err) => format!("{err:#?}"),
    }
}

fn set_time(svm: &mut LiteSVM, ts: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = ts;
    if clock.slot == 0 {
        clock.slot = 1;
    }
    svm.set_sysvar(&clock);
}

fn data(svm: &LiteSVM, key: &Address) -> Vec<u8> {
    svm.get_account(key).expect("account").data
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn i64_at(bytes: &[u8], at: usize) -> i64 {
    i64::from_le_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn token_amount(svm: &LiteSVM, key: &Address) -> u64 {
    u64_at(&data(svm, key), 64)
}

struct World {
    svm: LiteSVM,
    admin: Keypair,
    mint: Address,
}

fn boot(insurance: u64) -> World {
    let mut svm = LiteSVM::new()
        .with_sigverify(false)
        .with_blockhash_check(false);
    svm.add_program(ID, include_bytes!("../../target/deploy/mark_perp.so"))
        .unwrap();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    set_time(&mut svm, NOW);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let exchange = pda(&[b"exchange"]);
    let mut body = vec![1u8];
    for n in [1_000_000_000u64, 10_000_000_000, insurance, 1_000_000_000_000] {
        body.extend(n.to_le_bytes());
    }
    send(
        &mut svm,
        &admin,
        ix(
            "initialize_exchange",
            &body,
            vec![
                wr(admin.pubkey(), true),
                wr(exchange, false),
                wr(mint, true),
                wr(pda(&[b"vault"]), false),
                wr(pda(&[b"insurance"]), false),
                wr(pda(&[b"fees"]), false),
                ro(TOKEN),
                ro(SYSTEM),
            ],
        ),
        &[&mint_kp],
    );
    if insurance > 0 {
        send(
            &mut svm,
            &admin,
            ix(
                "seed_insurance",
                &insurance.to_le_bytes(),
                vec![
                    wr(admin.pubkey(), true),
                    wr(exchange, false),
                    wr(mint, false),
                    wr(pda(&[b"insurance"]), false),
                    ro(TOKEN),
                ],
            ),
            &[],
        );
    }
    World { svm, admin, mint }
}

fn push_str(buf: &mut Vec<u8>, text: &str) {
    buf.extend((text.len() as u32).to_le_bytes());
    buf.extend(text.as_bytes());
}

fn market_body(id: u16, index: u64, expiry: i64, max_age: u32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(id.to_le_bytes());
    push_str(&mut body, "SOL-WEEK");
    push_str(&mut body, "Will SOL finish the week above the listed strike?");
    push_str(&mut body, "The admin settles from the rule printed on the tape.");
    body.push(0); // admin oracle
    body.extend([0u8; 32]);
    body.extend(150_000_000u64.to_le_bytes());
    body.extend(expiry.to_le_bytes());
    body.extend(index.to_le_bytes());
    body.extend(0u64.to_le_bytes());
    body.extend(150_000_000u64.to_le_bytes());
    for n in [1_000u64, 1_000_000, 500, 1_000_000] {
        body.extend(n.to_le_bytes());
    }
    for n in [500u16, 5_000, 250, 2_500, 20] {
        body.extend(n.to_le_bytes());
    }
    for n in [1u64, 10, 50_000] {
        body.extend(n.to_le_bytes());
    }
    body.extend(3_600i64.to_le_bytes());
    body.extend((1_000_000 * SIZE).to_le_bytes());
    body.extend((100_000 * SIZE).to_le_bytes());
    for n in [20u16, 100, 2_000] {
        body.extend(n.to_le_bytes());
    }
    body.extend(max_age.to_le_bytes());
    body.extend(250u16.to_le_bytes());
    body
}

impl World {
    fn exchange(&self) -> Address {
        pda(&[b"exchange"])
    }
    fn vault(&self) -> Address {
        pda(&[b"vault"])
    }
    fn insurance(&self) -> Address {
        pda(&[b"insurance"])
    }
    fn fees(&self) -> Address {
        pda(&[b"fees"])
    }
    fn market_pda(&self, id: u16) -> Address {
        pda(&[b"market", &id.to_le_bytes()])
    }
    fn user(&self, owner: &Address) -> Address {
        pda(&[b"user", owner.as_ref()])
    }
    fn position(&self, market: &Address, owner: &Address) -> Address {
        pda(&[b"position", market.as_ref(), owner.as_ref()])
    }

    fn init_market(&mut self, id: u16, index: u64, expiry: i64, max_age: u32) -> Address {
        let market = self.market_pda(id);
        let admin = self.admin.pubkey();
        let exchange = self.exchange();
        let instruction = ix(
            "initialize_market",
            &market_body(id, index, expiry, max_age),
            vec![
                wr(admin, true),
                wr(exchange, false),
                wr(market, false),
                ro(SYSTEM),
            ],
        );
        send(&mut self.svm, &self.admin, instruction, &[]);
        market
    }

    fn push(&mut self, market: Address, index: u64) {
        let mut body = Vec::new();
        body.extend(index.to_le_bytes());
        body.extend(0u64.to_le_bytes());
        body.extend(150_000_000u64.to_le_bytes());
        let admin = self.admin.pubkey();
        let exchange = self.exchange();
        let instruction = ix(
            "push_admin_index",
            &body,
            vec![wr(admin, true), ro(exchange), wr(market, false)],
        );
        send(&mut self.svm, &self.admin, instruction, &[]);
    }

    fn onboard(&mut self, trader: &Keypair, amount: u64) {
        self.svm.airdrop(&trader.pubkey(), 2_000_000_000).unwrap();
        let user = self.user(&trader.pubkey());
        send(
            &mut self.svm,
            trader,
            ix(
                "initialize_user",
                &[],
                vec![wr(trader.pubkey(), true), wr(user, false), ro(SYSTEM)],
            ),
            &[],
        );
        let user_ata = ata_of(&trader.pubkey(), &self.mint);
        let exchange = self.exchange();
        let mint = self.mint;
        let faucet_ix = ix(
            "faucet",
            &amount.to_le_bytes(),
            vec![
                wr(trader.pubkey(), true),
                ro(exchange),
                wr(user, false),
                wr(mint, false),
                wr(user_ata, false),
                ro(TOKEN),
                ro(ATA),
                ro(SYSTEM),
            ],
        );
        send(&mut self.svm, trader, faucet_ix, &[]);
        let vault = self.vault();
        let deposit_ix = ix(
            "deposit_collateral",
            &amount.to_le_bytes(),
            vec![
                wr(trader.pubkey(), true),
                ro(exchange),
                wr(user, false),
                wr(user_ata, false),
                wr(vault, false),
                ro(mint),
                ro(TOKEN),
            ],
        );
        send(&mut self.svm, trader, deposit_ix, &[]);
    }

    fn open_ix(&self, trader: &Keypair, market: Address, side: u8, size: u64, limit: u64) -> Instruction {
        let mut body = vec![side];
        body.extend(size.to_le_bytes());
        body.extend(limit.to_le_bytes());
        ix(
            "open_or_increase",
            &body,
            vec![
                wr(trader.pubkey(), true),
                wr(self.exchange(), false),
                wr(market, false),
                wr(self.user(&trader.pubkey()), false),
                wr(self.position(&market, &trader.pubkey()), false),
                wr(self.vault(), false),
                wr(self.insurance(), false),
                wr(self.fees(), false),
                ro(TOKEN),
                ro(SYSTEM),
            ],
        )
    }

    fn open(&mut self, trader: &Keypair, market: Address, side: u8, size: u64, limit: u64) {
        let instruction = self.open_ix(trader, market, side, size, limit);
        send(&mut self.svm, trader, instruction, &[]);
    }

    fn close_ix(&self, trader: &Keypair, market: Address, size: u64, limit: u64) -> Instruction {
        let mut body = Vec::new();
        body.extend(size.to_le_bytes());
        body.extend(limit.to_le_bytes());
        ix(
            "reduce_or_close",
            &body,
            vec![
                wr(trader.pubkey(), true),
                wr(self.exchange(), false),
                wr(market, false),
                wr(self.user(&trader.pubkey()), false),
                wr(self.position(&market, &trader.pubkey()), false),
                wr(self.vault(), false),
                wr(self.insurance(), false),
                wr(self.fees(), false),
                ro(TOKEN),
            ],
        )
    }

    fn close(&mut self, trader: &Keypair, market: Address, size: u64, limit: u64) {
        let instruction = self.close_ix(trader, market, size, limit);
        send(&mut self.svm, trader, instruction, &[]);
    }

    fn free(&self, owner: &Address) -> u64 {
        u64_at(&data(&self.svm, &self.user(owner)), 40)
    }

    fn margin(&self, market: &Address, owner: &Address) -> u64 {
        u64_at(&data(&self.svm, &self.position(market, owner)), 88)
    }

    fn entry(&self, market: &Address, owner: &Address) -> u64 {
        u64_at(&data(&self.svm, &self.position(market, owner)), 80)
    }

    fn size(&self, market: &Address, owner: &Address) -> i64 {
        i64_at(&data(&self.svm, &self.position(market, owner)), 72)
    }

    fn status(&self, market: &Address) -> u8 {
        data(&self.svm, market)[587]
    }

    fn mark(&self, market: &Address) -> u64 {
        u64_at(&data(&self.svm, market), 596)
    }

    fn premium(&self, market: &Address) -> i64 {
        i64_at(&data(&self.svm, market), 653)
    }
}

#[test]
fn long_round_trip_then_funding_when_the_book_is_rich() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 500_000, NOW + 86_400 * 30, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 20_000_000);
    let size = 2 * SIZE;
    w.open(&trader, market, 0, size, u64::MAX);
    assert_eq!(w.entry(&market, &trader.pubkey()), 501_000);
    assert_eq!(w.margin(&market, &trader.pubkey()), 501_000);
    assert_eq!(w.free(&trader.pubkey()), 20_000_000 - 501_000 - 4_000);
    w.close(&trader, market, size, 0);
    assert_eq!(w.size(&market, &trader.pubkey()), 0);
    assert_eq!(w.free(&trader.pubkey()), 20_000_000 - 8_000);
    assert_eq!(token_amount(&w.svm, &w.insurance()), 50_000_000);

    w.open(&trader, market, 0, size, u64::MAX);
    let before = w.free(&trader.pubkey());
    set_time(&mut w.svm, NOW + 3_600);
    w.push(market, 500_000);
    send(
        &mut w.svm,
        &w.admin,
        ix("update_funding", &[], vec![wr(market, false)]),
        &[],
    );
    assert_eq!(w.mark(&market), 502_000);
    assert!(w.premium(&market) > 0);
    w.close(&trader, market, size, 0);
    assert_eq!(w.free(&trader.pubkey()), before + 501_000 - 4_000 - 400);
    assert_eq!(token_amount(&w.svm, &w.insurance()), 50_000_000 + 400);
}

#[test]
fn short_pays_when_the_mark_is_cheap() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 500_000, NOW + 86_400 * 30, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 20_000_000);
    let size = 2 * SIZE;
    w.open(&trader, market, 1, size, 0);
    assert_eq!(w.entry(&market, &trader.pubkey()), 499_000);
    assert!(w.size(&market, &trader.pubkey()) < 0);
    let margin = w.margin(&market, &trader.pubkey());
    set_time(&mut w.svm, NOW + 3_600);
    w.push(market, 500_000);
    send(
        &mut w.svm,
        &w.admin,
        ix("update_funding", &[], vec![wr(market, false)]),
        &[],
    );
    assert!(w.premium(&market) < 0);
    let before = w.free(&trader.pubkey());
    w.close(&trader, market, size, u64::MAX);
    assert_eq!(w.free(&trader.pubkey()), before + margin - 4_000 - 400);
}

#[test]
fn withdraw_stops_at_locked_margin() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 500_000, 0, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 1_000_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);
    let free = w.free(&trader.pubkey());
    let user_ata = ata_of(&trader.pubkey(), &w.mint);
    let withdraw = |amount: u64, trader: &Keypair, w: &World| {
        ix(
            "withdraw_collateral",
            &amount.to_le_bytes(),
            vec![
                wr(trader.pubkey(), true),
                ro(w.exchange()),
                wr(w.user(&trader.pubkey()), false),
                wr(user_ata, false),
                wr(w.vault(), false),
                ro(w.mint),
                ro(TOKEN),
            ],
        )
    };
    let too_much = withdraw(free + 1, &trader, &w);
    let err = send_err(&mut w.svm, &trader, too_much);
    assert!(err.to_lowercase().contains("locked") || err.contains("Withdraw"), "{err}");
    let ok = withdraw(free, &trader, &w);
    send(&mut w.svm, &trader, ok, &[]);
    assert_eq!(w.free(&trader.pubkey()), 0);
}

#[test]
fn stale_oracle_and_pause_and_a_thin_deposit() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 500_000, 0, 30);
    let trader = Keypair::new();
    w.onboard(&trader, 5_000_000);
    set_time(&mut w.svm, NOW + 31);
    let stale = w.open_ix(&trader, market, 0, SIZE, u64::MAX);
    let err = send_err(&mut w.svm, &trader, stale);
    assert!(err.to_lowercase().contains("stale"), "{err}");
    w.push(market, 500_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);

    let admin = w.admin.pubkey();
    let exchange = w.exchange();
    let pause = ix(
        "admin_pause",
        &[1u8],
        vec![wr(admin, true), wr(exchange, false)],
    );
    send(&mut w.svm, &w.admin, pause, &[]);
    let blocked = w.open_ix(&trader, market, 0, SIZE, u64::MAX);
    let err = send_err(&mut w.svm, &trader, blocked);
    assert!(err.to_lowercase().contains("paused"), "{err}");
    let size = w.size(&market, &trader.pubkey()).unsigned_abs();
    w.close(&trader, market, size, 0);
    assert_eq!(w.size(&market, &trader.pubkey()), 0);

    let thin = Keypair::new();
    // Pause is still on. Unpause, then reject a deposit that cannot cover margin.
    let admin = w.admin.pubkey();
    let exchange = w.exchange();
    let unpause = ix(
        "admin_pause",
        &[0u8],
        vec![wr(admin, true), wr(exchange, false)],
    );
    send(&mut w.svm, &w.admin, unpause, &[]);
    w.onboard(&thin, 1_000);
    let thin_open = w.open_ix(&thin, market, 0, SIZE, u64::MAX);
    let err = send_err(&mut w.svm, &thin, thin_open);
    assert!(err.to_lowercase().contains("free collateral"), "{err}");
}

#[test]
fn ninety_cent_long_posts_more_margin_than_fifty() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 900_000, 0, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 5_000_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);
    let at_90 = w.margin(&market, &trader.pubkey());
    let size = w.size(&market, &trader.pubkey()).unsigned_abs();
    w.close(&trader, market, size, 0);
    w.push(market, 500_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);
    let at_50 = w.margin(&market, &trader.pubkey());
    assert!(at_90 > at_50, "90¢ {at_90} vs 50¢ {at_50}");
}

#[test]
fn liquidation_pays_the_keeper_and_leaves_the_rest() {
    let mut w = boot(50_000_000);
    let market = w.init_market(0, 500_000, 0, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 5_000_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);
    let keeper = Keypair::new();
    w.svm.airdrop(&keeper.pubkey(), 2_000_000_000).unwrap();
    let healthy = w.liq_ix(&keeper, &trader, market);
    let err = send_err(&mut w.svm, &keeper, healthy);
    assert!(err.to_lowercase().contains("maintenance"), "{err}");

    w.push(market, 292_500);
    let free_before = w.free(&trader.pubkey());
    let instruction = w.liq_ix(&keeper, &trader, market);
    send(&mut w.svm, &keeper, instruction, &[]);
    assert_eq!(w.size(&market, &trader.pubkey()), 0);
    let bounty = token_amount(&w.svm, &ata_of(&keeper.pubkey(), &w.mint));
    assert!(bounty > 0, "keeper bounty");
    assert!(w.free(&trader.pubkey()) > free_before, "leftover returns");
}

#[test]
fn insurance_shortfall_pauses_and_expiry_settles() {
    let mut w = boot(5_000);
    let market = w.init_market(0, 500_000, NOW + 500, 20_000);
    let trader = Keypair::new();
    w.onboard(&trader, 5_000_000);
    w.open(&trader, market, 0, SIZE, u64::MAX);
    w.push(market, 800_000);
    let size = w.size(&market, &trader.pubkey()).unsigned_abs();
    w.close(&trader, market, size, 0);
    assert_eq!(w.status(&market), 1, "paused after a shortfall");
    assert_eq!(token_amount(&w.svm, &w.insurance()), 0);

    set_time(&mut w.svm, NOW + 501);
    w.push(market, 500_000);
    let ix = ix(
        "settle_expired",
        &0u64.to_le_bytes(),
        vec![
            wr(w.admin.pubkey(), true),
            ro(w.exchange()),
            wr(market, false),
        ],
    );
    send(&mut w.svm, &w.admin, ix, &[]);
    assert_eq!(w.status(&market), 3);
    assert_eq!(u64_at(&data(&w.svm, &market), 779), 1_000);
}

impl World {
    fn liq_ix(&self, keeper: &Keypair, trader: &Keypair, market: Address) -> Instruction {
        ix(
            "liquidate",
            &[],
            vec![
                wr(keeper.pubkey(), true),
                ro(trader.pubkey()),
                wr(self.exchange(), false),
                wr(market, false),
                wr(self.user(&trader.pubkey()), false),
                wr(self.position(&market, &trader.pubkey()), false),
                wr(self.vault(), false),
                wr(self.insurance(), false),
                ro(self.mint),
                wr(ata_of(&keeper.pubkey(), &self.mint), false),
                ro(TOKEN),
                ro(ATA),
                ro(SYSTEM),
            ],
        )
    }
}
