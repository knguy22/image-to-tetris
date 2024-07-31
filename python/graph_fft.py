import pandas as pd
import matplotlib.pyplot as plt

def main():
    df = pd.read_csv('fft.csv')

    # expected structure: channel, index, normalized value
    channels = df.groupby('channel')
    for channel, group in channels:
        sample = group['sample']
        norm = group['norm']

        plt.figure(figsize=(10, 6))
        plt.plot(sample, norm, 'b-', label='Normalized value')

        plt.xlabel('Sample')
        plt.ylabel('Value')
        plt.title(f'Channel {channel}: Normalized value after FFT')
        plt.legend()

        plt.grid(True)
        plt.savefig(f'fft_channel_{channel}.png')

if __name__ == "__main__":
    main()