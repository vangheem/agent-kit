#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("conflict")]
    Conflict,
    #[error("not_found")]
    NotFound,
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Sqlx(sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

impl DbError {
    pub(crate) fn from_sqlx(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(db_err) = &err
            && let Some(mapped) = map_db_kind(db_err.kind())
        {
            return mapped;
        }
        if matches!(err, sqlx::Error::RowNotFound) {
            return Self::NotFound;
        }
        Self::Sqlx(err)
    }
}

impl From<sqlx::Error> for DbError {
    fn from(err: sqlx::Error) -> Self {
        Self::from_sqlx(err)
    }
}

pub(crate) fn map_db_kind(kind: sqlx::error::ErrorKind) -> Option<DbError> {
    match kind {
        sqlx::error::ErrorKind::UniqueViolation => Some(DbError::Conflict),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_violation_maps_to_conflict() {
        assert!(matches!(
            map_db_kind(sqlx::error::ErrorKind::UniqueViolation),
            Some(DbError::Conflict)
        ));
        assert!(map_db_kind(sqlx::error::ErrorKind::ForeignKeyViolation).is_none());
    }
}
