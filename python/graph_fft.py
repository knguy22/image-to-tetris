import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import librosa
import os

def compute_local_average(x, M):
    """Compute local average of signal

    Notebook: C6/C6S1_NoveltySpectral.ipynb

    Args:
        x (np.ndarray): Signal
        M (int): Determines size (2M+1) in samples of centric window  used for local average

    Returns:
        local_average (np.ndarray): Local average signal
    """
    L = len(x)
    local_average = np.zeros(L)
    for m in range(L):
        a = max(m - M, 0)
        b = min(m + M + 1, L)
        local_average[m] = (1 / (2 * M + 1)) * np.sum(x[a:b])
    return local_average

def test_spectral_onset():
    # Raw STFT
    fn_wav = os.path.join('rick_input.wav')
    Fs = 44100
    x, Fs = librosa.load(path=fn_wav, sr=Fs)
    x_duration = len(x)/Fs
    N, H = 1024, 256
    print(x.shape)

    X = librosa.stft(x, n_fft=N, hop_length=H, win_length=N, window='hann')
    fig, ax = plt.subplots(3, 2, gridspec_kw={'width_ratios': [1, 0.05], 'height_ratios': [1, 1, 1]}, figsize=(6.5, 6))        
    Y = np.abs(X)

    # After performing half-wave rectification
    Y = np.log(1 + 100 * np.abs(X))
    Y_diff = np.diff(Y, n=1)
    Y_diff[Y_diff < 0] = 0
    nov = np.sum(Y_diff, axis=0)
    nov = np.concatenate((nov, np.array([0])))
    Fs_nov = Fs/H

    # local average
    M_sec = 0.1
    M = int(np.ceil(M_sec * Fs_nov))
    local_average = compute_local_average(nov, M)
    nov_norm =  nov - local_average
    nov_norm[nov_norm<0]=0
    nov_norm = nov_norm / max(nov_norm)

    print(X.shape, X[0], X[1])
    print(Y.shape, Y[0], Y[1])
    print(Y_diff.shape, Y_diff[0])
    print(nov.shape, nov[0], nov[1])
    print(nov_norm.shape, nov_norm[0], nov_norm[1])

    plt.figure(figsize=(10, 6))
    plt.plot(np.arange(0, nov_norm.shape[0]/Fs_nov, 1/Fs_nov), nov_norm, 'b-')
    plt.xlabel('Time (s)')
    plt.ylabel('Amplitude')
    plt.title('Input Signal')
    plt.savefig('input_signal.png')

def main():
    df = pd.read_csv('fft.csv')

    # expected structure: channel, index, normalized value
    channels = df.groupby('channel')
    for channel, group in channels:
        frequency = group['frequency']
        norm = group['norm']

        plt.figure(figsize=(10, 6))
        plt.plot(frequency, norm, 'b-', label='Normalized value')

        plt.xlabel('Frequency')
        plt.ylabel('Normalized Value')
        plt.xlim(0, 2000)
        plt.title(f'Channel {channel} after FFT')
        plt.legend()

        plt.grid(True)
        plt.savefig(f'fft_channel_{channel}.png')

if __name__ == "__main__":
    test_spectral_onset()
    # main()