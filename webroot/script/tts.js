// Text-to-Speech for Helipad
class TextToSpeech {
    constructor() {
        this.synth = window.speechSynthesis;
        this.voice = null;
        this.enabled = false;
        this.provider = 'browser';
        this.rate = 1.0;
        this.pitch = 1.0;
        this.volume = 1.0;
        this.currentText = null;

        // Amazon Polly settings
        this.awsVoiceId = 'Joanna';

        // Custom announcement scripts
        this.customScriptWithMessage = '';
        this.customScriptWithoutMessage = '';

        this.synth.onvoiceschanged = () => this.loadVoices();
        this.loadVoices();
    }

    loadVoices() {
        const voices = this.synth.getVoices();
        if (voices.length > 0) {
            this.voice = voices.find(v => v.lang === 'en-US') || voices[0];
        }
    }

    getVoices() {
        return this.synth.getVoices();
    }

    syncFromServer(settings) {
        if (!settings) return;

        this.enabled = settings.tts_enabled || false;
        this.provider = settings.tts_provider || 'browser';
        this.rate = parseFloat(settings.tts_rate) || 1.0;
        this.pitch = parseFloat(settings.tts_pitch) || 1.0;
        this.volume = settings.tts_volume !== undefined ? parseFloat(settings.tts_volume) : 1.0;
        this.customScriptWithMessage = settings.tts_script || '';
        this.customScriptWithoutMessage = settings.tts_script_without_message || '';
        this.awsVoiceId = settings.tts_aws_voice_id || 'Joanna';

        if (settings.tts_voice) {
            this.setVoice(settings.tts_voice);
        }
    }

    setVoice(voiceName) {
        const voices = this.synth.getVoices();
        this.voice = voiceName
            ? voices.find(v => v.name === voiceName) || voices.find(v => v.lang === 'en-US') || voices[0]
            : voices.find(v => v.lang === 'en-US') || voices[0];
    }

    setRate(rate) {
        this.rate = Math.max(0.1, Math.min(10, rate));
    }

    setPitch(pitch) {
        this.pitch = Math.max(0, Math.min(2, pitch));
    }

    setVolume(volume) {
        this.volume = Math.max(0, Math.min(1, volume));
    }

    async speak(text, voiceId) {
        if (!text) return;

        return this.provider === 'amazon_polly'
            ? this.speakWithPolly(text, voiceId)
            : this.speakWithBrowser(text);
    }

    async speakWithBrowser(text) {
        return new Promise((resolve) => {
            this.synth.cancel();

            const utterance = new SpeechSynthesisUtterance(text);
            utterance.voice = this.voice;
            utterance.rate = this.rate;
            utterance.pitch = this.pitch;
            utterance.volume = this.volume;

            utterance.onend = () => {
                this.currentText = null;
                resolve();
            };

            this.synth.speak(utterance);
            this.currentText = text;
        });
    }

    async speakWithPolly(text, voiceId) {
        try {
            const requestBody = {
                text,
                voice_id: voiceId || this.awsVoiceId
            };

            const response = await fetch('/api/v1/polly/tts', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(requestBody)
            });

            const data = await response.json();

            if (!data.success || !data.audio_data) {
                console.error('Amazon Polly TTS error:', data.error || 'Unknown error');
                return;
            }

            await this.playAudioData(data.audio_data, text);
        } catch (error) {
            console.error('Amazon Polly TTS request failed:', error);
        }
    }

    async playAudioData(base64Audio, text) {
        const audioData = atob(base64Audio);
        const audioArray = new Uint8Array(audioData.length);
        for (let i = 0; i < audioData.length; i++) {
            audioArray[i] = audioData.charCodeAt(i);
        }

        const audioBlob = new Blob([audioArray], { type: 'audio/mpeg' });
        const audioUrl = URL.createObjectURL(audioBlob);
        const audio = new Audio(audioUrl);
        audio.volume = this.volume;

        return new Promise((resolve) => {
            audio.onended = () => {
                URL.revokeObjectURL(audioUrl);
                this.currentText = null;
                resolve();
            };
            audio.onerror = (e) => {
                console.error('Error playing Amazon Polly audio:', e);
                URL.revokeObjectURL(audioUrl);
                resolve();
            };
            audio.play();
            this.currentText = text;
        });
    }

    stop() {
        this.synth.cancel();
        this.currentText = null;
    }

    shouldAnnounce(boost, type, settings) {
        if (!this.enabled || !settings) return false;

        const typeSettings = {
            boost: settings.tts_on_boost,
            stream: settings.tts_on_stream,
            payment: settings.tts_on_payment
        };

        if (!typeSettings[type]) return false;

        if (settings.tts_min_sats) {
            const sats = Math.trunc((boost.value_msat_total || boost.value_msat) / 1000);
            if (sats < settings.tts_min_sats) return false;
        }

        return true;
    }

    formatText(boost, type, customScript) {
        const defaultScript = '{sender} sent {sats} sats and says: {message}';
        const script = customScript ||
            (boost.message ? this.customScriptWithMessage : this.customScriptWithoutMessage) ||
            defaultScript;

        const sats = Math.trunc((boost.value_msat_total || boost.value_msat || 0) / 1000);
        const replacements = {
            type: type || 'payment',
            sats,
            sender: boost.sender || 'anonymous',
            podcast: boost.podcast || '',
            episode: boost.episode || '',
            app: boost.app || '',
            message: boost.message || '',
            message_preview: boost.message ? boost.message.substring(0, 50) : ''
        };

        let text = script;
        for (const [key, value] of Object.entries(replacements)) {
            text = text.replace(new RegExp(`\\{${key}\\}`, 'g'), value);
        }

        return text.replace(/\s+/g, ' ').trim();
    }

    announceBoost(boost, type, customScript) {
        const text = this.formatText(boost, type, customScript);
        return this.speak(text);
    }
}

// Create global instance
window.helipadTTS = new TextToSpeech();