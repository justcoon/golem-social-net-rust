# Performance Benchmarks

This directory contains benchmarking tools and configurations for testing the performance of the Golem Social Net application.

## Benchmarking Tools

- **[Goose](https://github.com/tag1consulting/goose)**: Used for load testing the application
- **[Drill](https://github.com/fcsonline/drill)**: Used for importing test data

## Prerequisites

1. Install required tools:
   ```bash
   # Install Goose
   cargo install cargo-goose
   
   # Install Drill
   cargo install drill
   ```

2. Ensure the Golem Social Net application is running

## Environment Variables

- `HOST`: Worker service API gateway host (e.g., `http://localhost:9006`)
- `API_HOST`: API deployment host/site (e.g., `http://localhost:9006`)

## Importing Test Data

Before running benchmarks, you'll need to import test [data](../data/README.md) using

## Running Benchmarks

To run the load tests:

```bash
cd benchmark
HOST=http://localhost:9006 API_HOST=localhost:9006 cargo run --release -- --report-file=report.html --no-reset-metrics
```

### Test Coverage

The benchmark tests the following components:
- users (IDs: u001 - u100)


## Understanding the Results

After running the benchmarks, a report will be generated at `report.html`. This report includes:

- Response times (min, max, average)
- Requests per second
- Error rates
- Detailed metrics for each endpoint

## Customizing Benchmarks

You can customize the benchmark parameters by modifying:

- `src/main.rs`: Adjust test scenarios and load patterns

## Troubleshooting

- Ensure all services are running before starting benchmarks
- Verify environment variables are correctly set
- Check for port conflicts if benchmarks fail to start
- Review the generated report for detailed error information