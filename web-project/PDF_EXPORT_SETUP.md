# Invoice PDF Export Setup

The invoice download route runs the Python executable in `PDF_PYTHON`.
When that variable is unset, it uses `python3`.

## Debian Dev Container

Install Python's virtual-environment support and create a local environment:

```bash
sudo apt-get update
sudo apt-get install -y python3-venv python3-pip
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
```

Start the application with that environment's Python:

```bash
PDF_PYTHON="$PWD/.venv/bin/python" cargo run
```

## Verify

```bash
.venv/bin/python -c "import reportlab; print(reportlab.Version)"
```

Open an invoice and select **Download PDF**. The browser should download a file named after the invoice number.
