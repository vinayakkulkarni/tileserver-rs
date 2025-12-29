//! PostgreSQL connection pool with PostGIS support.

use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod, Timeouts};
use semver::Version;
use std::path::PathBuf;
use std::time::Duration;
use tokio_postgres::NoTls;

use crate::error::{Result, TileServerError};

use super::{MINIMUM_POSTGIS_VERSION, MINIMUM_POSTGRES_VERSION, ST_TILE_ENVELOPE_MARGIN_VERSION};

#[derive(Clone, Debug, Default)]
pub struct PoolSettings {
    pub max_size: usize,
    pub wait_timeout_ms: u64,
    pub create_timeout_ms: u64,
    pub recycle_timeout_ms: u64,
}

#[derive(Clone, Debug)]
pub struct PostgresPool {
    id: String,
    pool: Pool,
    supports_tile_margin: bool,
    postgres_version: Version,
    postgis_version: Version,
}

impl PostgresPool {
    pub async fn new(
        connection_string: &str,
        settings: PoolSettings,
        _ssl_cert: Option<&PathBuf>,
        _ssl_key: Option<&PathBuf>,
        _ssl_root_cert: Option<&PathBuf>,
    ) -> Result<Self> {
        let pg_config: tokio_postgres::Config = connection_string.parse().map_err(|e| {
            TileServerError::PostgresError(format!("Invalid connection string: {}", e))
        })?;

        let id = pg_config
            .get_dbname()
            .map(ToString::to_string)
            .unwrap_or_else(|| "postgres".to_string());

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let mgr = Manager::from_config(pg_config, NoTls, mgr_config);

        let timeouts = Timeouts {
            wait: Some(Duration::from_millis(settings.wait_timeout_ms)),
            create: Some(Duration::from_millis(settings.create_timeout_ms)),
            recycle: Some(Duration::from_millis(settings.recycle_timeout_ms)),
        };

        let pool = Pool::builder(mgr)
            .max_size(settings.max_size)
            .timeouts(timeouts)
            .build()
            .map_err(|e| {
                TileServerError::PostgresPoolError(format!("Failed to build pool: {}", e))
            })?;

        let mut result = Self {
            id: id.clone(),
            pool,
            supports_tile_margin: false,
            postgres_version: Version::new(0, 0, 0),
            postgis_version: Version::new(0, 0, 0),
        };

        // Get a connection to check versions
        let conn = result.get().await?;

        // Check PostgreSQL version
        let pg_version = Self::get_postgres_version(&conn).await?;
        if pg_version < MINIMUM_POSTGRES_VERSION {
            return Err(TileServerError::PostgresVersionError(format!(
                "PostgreSQL {} is older than minimum required {}",
                pg_version, MINIMUM_POSTGRES_VERSION
            )));
        }

        // Check PostGIS version
        let postgis_version = Self::get_postgis_version(&conn).await?;
        if postgis_version < MINIMUM_POSTGIS_VERSION {
            return Err(TileServerError::PostgresVersionError(format!(
                "PostGIS {} is older than minimum required {}",
                postgis_version, MINIMUM_POSTGIS_VERSION
            )));
        }

        result.postgres_version = pg_version;
        result.postgis_version = postgis_version.clone();
        result.supports_tile_margin = postgis_version >= ST_TILE_ENVELOPE_MARGIN_VERSION;

        if !result.supports_tile_margin {
            tracing::warn!(
                "PostGIS {} is older than {}. Margin parameter in ST_TileEnvelope is not supported.",
                postgis_version,
                ST_TILE_ENVELOPE_MARGIN_VERSION
            );
        }

        tracing::info!(
            "Connected to PostgreSQL {} / PostGIS {} for source {}",
            result.postgres_version,
            result.postgis_version,
            id
        );

        Ok(result)
    }

    /// Gets a connection from the pool.
    pub async fn get(&self) -> Result<Object> {
        self.pool.get().await.map_err(|e| {
            TileServerError::PostgresPoolError(format!("Failed to get connection: {}", e))
        })
    }

    /// Returns the pool identifier (database name).
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns whether ST_TileEnvelope supports the margin parameter.
    pub fn supports_tile_margin(&self) -> bool {
        self.supports_tile_margin
    }

    /// Returns the PostgreSQL version.
    pub fn postgres_version(&self) -> &Version {
        &self.postgres_version
    }

    /// Returns the PostGIS version.
    pub fn postgis_version(&self) -> &Version {
        &self.postgis_version
    }

    /// Gets the PostgreSQL server version.
    async fn get_postgres_version(conn: &Object) -> Result<Version> {
        let row = conn
            .query_one(
                r"SELECT (regexp_matches(
                    current_setting('server_version'),
                    '^(\d+\.\d+)',
                    'g'
                ))[1] || '.0' as version;",
                &[],
            )
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to query PostgreSQL version: {}", e))
            })?;

        let version_str: String = row.get("version");
        version_str.parse().map_err(|e| {
            TileServerError::PostgresVersionError(format!(
                "Invalid PostgreSQL version '{}': {}",
                version_str, e
            ))
        })
    }

    /// Gets the PostGIS library version.
    async fn get_postgis_version(conn: &Object) -> Result<Version> {
        let row = conn
            .query_one(
                r"SELECT (regexp_matches(
                    PostGIS_Lib_Version(),
                    '^(\d+\.\d+\.\d+)',
                    'g'
                ))[1] as version;",
                &[],
            )
            .await
            .map_err(|e| {
                TileServerError::PostgresError(format!("Failed to query PostGIS version: {}", e))
            })?;

        let version_str: String = row.get("version");
        version_str.parse().map_err(|e| {
            TileServerError::PostgresVersionError(format!(
                "Invalid PostGIS version '{}': {}",
                version_str, e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let v11 = Version::new(11, 0, 0);
        let v12 = Version::new(12, 0, 0);
        let v3_0 = Version::new(3, 0, 0);
        let v3_1 = Version::new(3, 1, 0);

        assert!(v11 >= MINIMUM_POSTGRES_VERSION);
        assert!(v12 >= MINIMUM_POSTGRES_VERSION);
        assert!(v3_0 >= MINIMUM_POSTGIS_VERSION);
        assert!(v3_1 >= ST_TILE_ENVELOPE_MARGIN_VERSION);
        assert!(!(v3_0 >= ST_TILE_ENVELOPE_MARGIN_VERSION));
    }

    #[test]
    fn test_version_parsing() {
        let v: Version = "14.5.0".parse().unwrap();
        assert_eq!(v.major, 14);
        assert_eq!(v.minor, 5);
        assert_eq!(v.patch, 0);

        let v: Version = "3.4.2".parse().unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 4);
        assert_eq!(v.patch, 2);
    }
}
