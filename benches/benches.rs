use ascom_alpaca::benches;
use criterion::criterion_main;

criterion_main!(benches::client);
