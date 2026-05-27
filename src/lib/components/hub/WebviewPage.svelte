<script lang="ts">
  interface Props {
    page: 'gpt' | 'claude' | 'translate'
    url?: string
  }
  let { page, url = '' }: Props = $props()
</script>

{#if page === 'gpt'}
  <div class="hub-wv-mock hub-gpt">
    <div class="top">
      <div class="logo">G</div>
      <div class="model">ChatGPT <span class="ver">5</span></div>
      <div class="right">
        <div class="pill">Bagikan</div>
        <div class="pill" style="background:#0d0d0d;color:#fff">Upgrade ke Plus</div>
      </div>
    </div>
    <div class="stage">
      <div class="h">Apa yang ingin kamu kerjakan hari ini?</div>
      <div class="composer">
        <div class="plh">Tanyakan apa saja…</div>
        <div class="icons">
          <div class="ic">+</div>
          <div class="ic">⚙</div>
          <div class="ic">🎤</div>
          <div class="send">↑</div>
        </div>
      </div>
      <div class="prompts">
        <div class="p">Bantu aku menulis</div>
        <div class="p">Buat gambar</div>
        <div class="p">Analisis data</div>
        <div class="p">Buatkan rencana</div>
        <div class="p">Lebih banyak</div>
      </div>
    </div>
  </div>

{:else if page === 'claude'}
  <div class="hub-wv-mock hub-claude">
    <div class="top">
      <div class="logo">C</div>
      <div class="name">Claude</div>
      <div class="right">
        <div class="credit">Sonnet 4.5</div>
      </div>
    </div>
    <div class="stage">
      <div class="h">Selamat, Adi</div>
      <div class="composer">
        <div class="plh">Bagaimana saya bisa membantumu hari ini?</div>
        <div class="row">
          <div class="chip">Claude Sonnet 4.5</div>
          <div class="chip">+ Pilih gaya</div>
          <div class="send">↑</div>
        </div>
      </div>
      <div class="prompts">
        <div class="p">📝 Tulis</div>
        <div class="p">💡 Pelajari</div>
        <div class="p">💻 Kode</div>
        <div class="p">🎨 Hidup</div>
      </div>
    </div>
  </div>

{:else if page === 'translate'}
  <div class="hub-wv-mock hub-gt">
    <div class="top">
      <div class="logo">
        <span class="g">G</span>oogle Terjemahan
      </div>
      <div class="tabs">
        <div class="tab active">Teks</div>
        <div class="tab">Gambar</div>
        <div class="tab">Dokumen</div>
        <div class="tab">Situs</div>
      </div>
    </div>
    <div class="stage">
      <div class="lang-bar">
        <div class="seg">
          <div class="l-btn active">Indonesia</div>
          <div class="l-btn">Inggris</div>
          <div class="l-btn">Deteksi bahasa</div>
        </div>
        <div class="swap">⇄</div>
        <div class="seg">
          <div class="l-btn">Indonesia</div>
          <div class="l-btn active">Inggris</div>
          <div class="l-btn">Jepang</div>
        </div>
      </div>
      <div class="editor">
        <div class="pane">
          <div class="src-text">Kami sedang mengembangkan aplikasi AI desktop yang membantu produktivitas tim.</div>
          <div class="row-b">
            <div>🔊</div>
            <div>🎤</div>
            <div style="margin-left:auto;font-size:12px">82 / 5.000</div>
          </div>
        </div>
        <div class="pane r">
          <div class="out-text">We are developing a desktop AI application that enhances team productivity.</div>
          <div class="row-b">
            <div>🔊</div>
            <div>📋</div>
            <div>★</div>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}
