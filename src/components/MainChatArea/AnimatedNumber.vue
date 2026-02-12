<script setup lang="ts">
  import { ref, watch, onMounted, onUnmounted } from 'vue'

  interface Props {
    value: number
    prefix?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    prefix: '',
  })

  const displayValue = ref(0)
  let animationId: number | null = null

  const formatNumber = (num: number): string => {
    return num.toLocaleString()
  }

  const easeOutQuad = (t: number): number => 1 - (1 - t) * (1 - t)
  const easeOutCubic = (t: number): number => 1 - Math.pow(1 - t, 3)

  const clamp = (value: number, min: number, max: number) => {
    return Math.min(Math.max(value, min), max)
  }

  const cancelAnimation = () => {
    if (animationId !== null) {
      cancelAnimationFrame(animationId)
      animationId = null
    }
  }

  // Two-phase animation: fast then slow, last few numbers noticeably slow down
  const animateToValue = (target: number) => {
    cancelAnimation()

    const start = displayValue.value
    if (start === target) return

    const diff = target - start
    const absDiff = Math.abs(diff)

    const slowSpan = absDiff <= 40 ? absDiff : Math.max(40, Math.floor(absDiff * 0.35))
    const slowDirection = Math.sign(diff)
    const fastTarget = target - slowDirection * slowSpan

    const fastDuration = clamp(160 + Math.log10(absDiff + 1) * 120, 180, 420)
    const slowDuration = clamp(1200 + Math.log10(absDiff + 1) * 520, 1400, 2800)
    const startTime = performance.now()

    const animate = (now: number) => {
      const elapsed = now - startTime

      if (elapsed < fastDuration) {
        const t = elapsed / fastDuration
        const eased = easeOutQuad(t)
        displayValue.value = Math.round(start + (fastTarget - start) * eased)
      } else {
        const t = Math.min((elapsed - fastDuration) / slowDuration, 1)
        const eased = easeOutCubic(t)
        displayValue.value = Math.round(fastTarget + (target - fastTarget) * eased)
      }

      if (elapsed < fastDuration + slowDuration) {
        animationId = requestAnimationFrame(animate)
      } else {
        displayValue.value = target
        animationId = null
      }
    }

    animationId = requestAnimationFrame(animate)
  }

  // Watch value changes
  watch(
    () => props.value,
    newVal => {
      animateToValue(newVal)
    }
  )

  onMounted(() => {
    requestAnimationFrame(() => {
      animateToValue(props.value)
    })
  })

  onUnmounted(() => {
    cancelAnimation()
  })
</script>

<template>
  <span class="animated-number">
    <span class="prefix">{{ prefix }}</span>
    <span class="number">{{ formatNumber(displayValue) }}</span>
  </span>
</template>

<style scoped>
  .animated-number {
    display: inline-flex;
    font-variant-numeric: tabular-nums;
    color: inherit;
  }

  .prefix {
    color: inherit;
  }

  .number {
    color: inherit;
  }
</style>
