import csv
import random
import sys

def generate_signals(iters, num_signals, root_rate, prefix):
    """
    Generate multiple two-level signals (-1.0 or 1.0) with a specified root rate.

    root_rate = expected number of sign flips per 100 samples.
    From this, the flip probability is:
        flip_prob = root_rate / 100
    """

    num_samples = 601
    dt = 0.01

    # Convert root rate to probability of flipping
    flip_prob = root_rate / 100.0

    # Format rr for filename (avoid decimal point issues)
    rr_str = f"{root_rate}".replace(".", "_")

    for i in range(iters):
        for n in range(1, num_signals + 1):
            filename = f"{prefix}/sig_{i}_{rr_str}_{n}.csv"

            t = 0.0
            x_prev = random.choice([-1.0, 1.0])

            with open(filename, "w", newline="") as csvfile:
                writer = csv.writer(csvfile)

                for _ in range(num_samples):
                    writer.writerow([round(t, 2), x_prev])

                    # Determine next value
                    if random.random() <= flip_prob:
                        x_prev = -x_prev  # flip

                    t += dt

            # print(f"Generated: {filename}")


if __name__ == "__main__":
    if len(sys.argv) != 5:
        print("Usage: python datagen.py <iterations> <num_signals> <root_rate> <prefix>")
        sys.exit(1)

    iters = int(sys.argv[1])
    num_signals = int(sys.argv[2])
    root_rate = float(sys.argv[3])
    prefix = sys.argv[4]

    generate_signals(iters, num_signals, root_rate, prefix)
