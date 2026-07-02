# Release helpers for the Aircast QGroundControl fork.
# Tags pushed by these targets trigger .github/workflows/aircast-release.yml.

.PHONY: release.patch release.minor release.major release.dev release.staging

release.patch:
	./scripts/release.sh patch

release.minor:
	./scripts/release.sh minor

release.major:
	./scripts/release.sh major

release.dev:
	./scripts/release.sh dev

release.staging:
	./scripts/release.sh staging
