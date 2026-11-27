<script lang="ts">
  interface Props {
    size?: number;
  }

  let { size = 28 }: Props = $props();
</script>

<img
  src="/icon.png"
  alt=""
  width={size}
  height={size}
  decoding="async"
  class="shrink-0 select-none"
/>
