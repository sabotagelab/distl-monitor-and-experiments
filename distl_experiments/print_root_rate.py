import csv
import sys

def compute_root_rate(filename):
    """
    Compute the actual root rate from a CSV file containing time,value pairs.
    Root rate = (number of sign flips) / (number of 100-sample blocks)
    """

    values = []

    # Read values from CSV
    with open(filename, "r") as csvfile:
        reader = csv.reader(csvfile)
        for row in reader:
            values.append(float(row[1]))

    # Count sign flips
    flips = 0
    for i in range(1, len(values)):
        if values[i] != values[i - 1]:
            flips += 1

    # Number of 100-sample intervals
    intervals = (len(values)-1) / 100.0

    root_rate = flips / intervals

    return flips, root_rate


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python print_root_rate.py <csv_file>")
        sys.exit(1)

    filename = sys.argv[1]
    flips, rr = compute_root_rate(filename)

    print(f"File: {filename}")
    print(f"Sign flips: {flips}")
    print(f"Actual root rate: {rr:.4f}")
