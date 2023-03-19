#[derive(serde::Serialize)]
#[serde(untagged)]
pub(crate) enum Either<L, R> {
    Left(L),
    Right(R),
}
