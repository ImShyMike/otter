<p align="center">
    <img alt="Search engine for all Hack Club projects!" src="assets/hero.png" />
</p>
<h1 align="center" style="font-size: 52px; font-family: 'system-ui';">Otter</h1>
<p align="center">
    <a href="https://github.com/ImShyMike/otter/actions"><img alt="GitHub branch check runs" src="https://img.shields.io/github/check-runs/ImShyMike/otter/main"></a>
    <a href="https://github.com/ImShyMike/otter/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/ImShyMike/otter?style=flat&logo=github" /></a>
    <a href="https://github.com/ImShyMike/otter/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/ImShyMike/otter" /></a>
    <a href="https://github.com/ImShyMike/otter"><img alt="GitHub repo size" src="https://img.shields.io/github/repo-size/ImShyMike/otter"></a>
    <a href="https://github.com/ImShyMike/otter/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/github/license/ImShyMike/otter" /></a>
</p>

An easy-to-use search engine/API for all Hack Club projects, built with Rust, Svelte and Postgres. Projects are ingested from the [Ships API](https://github.com/hackclub/ships) and [Airbridge](https://github.com/hackclub/airbridge), and are then indexed and stored in the database.

## API Documentation

Documentation for the API can be found at [https://otter.shymike.dev/docs](https://otter.shymike.dev/docs)! (or [http://localhost:3000/docs](http://localhost:3000/docs) when running locally)

## Development

Make sure you have [Docker](https://www.docker.com), [Rust](https://www.rust-lang.org) and [Bun](https://bun.sh) installed.

```bash
# Start Postgres and Redis
docker compose up -d

# Start the backend
cd app
cargo run

# Start the frontend
cd frontend
bun i
bun dev
```

These should now be live:

- **frontend**: [http://localhost:5173](http://localhost:5173)
- **backend**: [http://localhost:3000](http://localhost:3000)

## Star History

<a href="https://www.star-history.com/?repos=ImShyMike%2Fotter&type=date&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=ImShyMike/otter&type=date&theme=dark&legend=top-left" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=ImShyMike/otter&type=date&legend=top-left" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=ImShyMike/otter&type=date&legend=top-left" />
 </picture>
</a>
