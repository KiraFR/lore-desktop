<script lang="ts">
  import * as THREE from 'three'
  import { OrbitControls } from 'three/addons/controls/OrbitControls.js'
  import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
  import { OBJLoader } from 'three/addons/loaders/OBJLoader.js'
  import { FBXLoader } from 'three/addons/loaders/FBXLoader.js'

  let { url, name }: { url: string; name: string } = $props()

  let container = $state<HTMLDivElement>()
  let failed = $state(false)

  // Full three.js lifecycle per (container, url): build, load, orbit, dispose.
  $effect(() => {
    const el = container
    const u = url
    if (!el) return
    let stop = false

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true })
    renderer.setPixelRatio(window.devicePixelRatio)
    el.appendChild(renderer.domElement)

    const scene = new THREE.Scene()
    const camera = new THREE.PerspectiveCamera(45, 1, 0.01, 1000)
    camera.position.set(2.2, 1.6, 2.8)
    scene.add(new THREE.HemisphereLight(0xffffff, 0x3a4148, 1.6))
    const key = new THREE.DirectionalLight(0xffffff, 2.2)
    key.position.set(3, 5, 4)
    scene.add(key)
    const grid = new THREE.GridHelper(4, 8, 0x3a4250, 0x2a2f36)
    ;(grid.material as THREE.Material).transparent = true
    ;(grid.material as THREE.Material).opacity = 0.4
    scene.add(grid)

    const controls = new OrbitControls(camera, renderer.domElement)
    controls.enableDamping = true
    // Turntable until the artist takes over.
    controls.autoRotate = true
    controls.autoRotateSpeed = 1.6
    controls.addEventListener('start', () => (controls.autoRotate = false))

    // Center the object, back the camera off its bounding box, seat the grid.
    function fit(obj: THREE.Object3D) {
      if (stop) return
      obj.traverse((c) => {
        const mesh = c as THREE.Mesh
        if (mesh.isMesh && mesh.geometry && !mesh.geometry.getAttribute('normal')) {
          mesh.geometry.computeVertexNormals()
        }
      })
      const box = new THREE.Box3().setFromObject(obj)
      const size = box.getSize(new THREE.Vector3())
      const center = box.getCenter(new THREE.Vector3())
      obj.position.sub(center)
      const maxDim = Math.max(size.x, size.y, size.z) || 1
      const dist = maxDim * 1.8
      camera.position.set(dist, dist * 0.7, dist)
      camera.near = maxDim / 100
      camera.far = maxDim * 20
      camera.updateProjectionMatrix()
      controls.target.set(0, 0, 0)
      grid.scale.setScalar(maxDim / 2)
      grid.position.y = -size.y / 2
      controls.update()
      scene.add(obj)
    }

    const ext = name.split('.').pop()?.toLowerCase() ?? ''
    const onErr = () => { if (!stop) failed = true }
    if (ext === 'glb' || ext === 'gltf') new GLTFLoader().load(u, (g) => fit(g.scene), undefined, onErr)
    else if (ext === 'fbx') new FBXLoader().load(u, fit, undefined, onErr)
    else if (ext === 'obj') new OBJLoader().load(u, fit, undefined, onErr)
    else failed = true

    const resize = () => {
      const w = el.clientWidth
      const h = el.clientHeight
      if (!w || !h) return
      renderer.setSize(w, h)
      camera.aspect = w / h
      camera.updateProjectionMatrix()
    }
    const ro = new ResizeObserver(resize)
    ro.observe(el)
    resize()

    renderer.setAnimationLoop(() => {
      controls.update()
      renderer.render(scene, camera)
    })

    return () => {
      stop = true
      ro.disconnect()
      renderer.setAnimationLoop(null)
      controls.dispose()
      scene.traverse((c) => {
        const mesh = c as THREE.Mesh
        if (mesh.isMesh) {
          mesh.geometry?.dispose()
          const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material]
          mats.forEach((m) => m?.dispose())
        }
      })
      renderer.dispose()
      renderer.domElement.remove()
    }
  })
</script>

{#if failed}
  <div class="fail muted"><p>Couldn't load this model.</p></div>
{:else}
  <div class="viewer" bind:this={container}></div>
{/if}

<style>
  .viewer { height: 340px; border: 1px solid var(--border); border-radius: 10px; overflow: hidden; background: radial-gradient(ellipse at 50% 40%, #232830, #17191d); }
  .viewer :global(canvas) { display: block; }
  .fail { padding: 22px; border: 1px dashed var(--border); border-radius: 8px; font-size: 12.5px; }
  .fail p { margin: 0; }
</style>
