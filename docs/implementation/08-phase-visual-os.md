# Phase 8: Visual OS

**Duration:** 12-16 weeks  
**Status:** Implementation Phase  
**Prerequisites:** Phase 7 (Bare Metal)

---

## Objective

Implement the graphical user interface subsystem following the "meaning is deterministic, appearance is nondeterministic" principle, with GPU acceleration and Axiom integration.

---

## Deliverables

### 8.1 Scene Graph

| Component | Description | Complexity |
|-----------|-------------|------------|
| Node structure | Scene node types | Medium |
| Tree operations | Add/remove/modify | Medium |
| Layout engine | Flex-based layout | High |
| Diffing | Incremental updates | Medium |

### 8.2 Input Handling

| Component | Description | Complexity |
|-----------|-------------|------------|
| Event processing | Mouse/keyboard events | Medium |
| Hit testing | Find target nodes | Medium |
| Focus management | Focus traversal | Medium |
| Semantic actions | Generate actions | Low |

### 8.3 Renderer

| Component | Description | Complexity |
|-----------|-------------|------------|
| GPU abstraction | wgpu integration | High |
| Render pipeline | Draw commands | High |
| Text rendering | Font rasterization | High |
| Image rendering | Texture loading | Medium |

### 8.4 Compositor

| Component | Description | Complexity |
|-----------|-------------|------------|
| Window management | Multiple windows | High |
| Layer compositing | Z-order handling | Medium |
| Damage tracking | Partial updates | Medium |

### 8.5 Axiom Integration

| Component | Description | Complexity |
|-----------|-------------|------------|
| Scene commit | Scene state to Axiom | Medium |
| Action flow | Actions through Axiom | Medium |
| Replay | Deterministic replay | Medium |

---

## Technical Approach

### Scene Graph

```rust
pub struct SceneGraph {
    root: NodeId,
    nodes: BTreeMap<NodeId, SceneNode>,
    version: u64,
}

impl SceneGraph {
    pub fn update(&mut self, changes: Vec<SceneChange>) -> SceneDiff {
        let mut diff = SceneDiff::new(self.version, self.version + 1);
        
        for change in changes {
            match change {
                SceneChange::AddNode { parent, node } => {
                    let id = node.id;
                    self.nodes.insert(id, node.clone());
                    
                    if let Some(parent_node) = self.nodes.get_mut(&parent) {
                        parent_node.children.push(id);
                    }
                    
                    diff.added.push(node);
                }
                SceneChange::RemoveNode { id } => {
                    self.remove_subtree(id);
                    diff.removed.push(id);
                }
                SceneChange::ModifyNode { id, changes } => {
                    if let Some(node) = self.nodes.get_mut(&id) {
                        for change in &changes {
                            node.apply_change(change);
                        }
                        diff.modified.push(NodeModification { node_id: id, changes });
                    }
                }
            }
        }
        
        self.version += 1;
        diff
    }
}
```

### Layout Engine

```rust
pub struct LayoutEngine;

impl LayoutEngine {
    pub fn layout(&self, scene: &mut SceneGraph, viewport: Size) {
        // Start from root
        let root = scene.root;
        self.layout_node(scene, root, Rect::from_size(viewport));
    }
    
    fn layout_node(&self, scene: &mut SceneGraph, node_id: NodeId, available: Rect) {
        let node = scene.nodes.get_mut(&node_id).unwrap();
        
        // Calculate this node's size
        let size = self.calculate_size(node, available.size());
        node.layout.bounds = Some(Rect::new(available.origin(), size));
        
        // Layout children based on flex properties
        if let Some(flex) = &node.layout.flex {
            self.layout_flex_children(scene, node_id, flex.clone());
        } else {
            self.layout_block_children(scene, node_id);
        }
    }
    
    fn layout_flex_children(&self, scene: &mut SceneGraph, parent_id: NodeId, flex: FlexLayout) {
        let parent = scene.nodes.get(&parent_id).unwrap();
        let bounds = parent.layout.bounds.unwrap();
        let children: Vec<NodeId> = parent.children.clone();
        
        // Calculate flex distribution
        let total_flex: f32 = children.iter()
            .filter_map(|id| scene.nodes.get(id))
            .map(|n| n.layout.width.flex_grow())
            .sum();
        
        let available_space = match flex.direction {
            FlexDirection::Row => bounds.width,
            FlexDirection::Column => bounds.height,
        };
        
        let mut offset = 0.0;
        for child_id in children {
            let child = scene.nodes.get(&child_id).unwrap();
            let flex_grow = child.layout.width.flex_grow();
            let child_size = (flex_grow / total_flex) * available_space;
            
            let child_bounds = match flex.direction {
                FlexDirection::Row => Rect::new(
                    Point::new(bounds.x + offset, bounds.y),
                    Size::new(child_size, bounds.height),
                ),
                FlexDirection::Column => Rect::new(
                    Point::new(bounds.x, bounds.y + offset),
                    Size::new(bounds.width, child_size),
                ),
            };
            
            self.layout_node(scene, child_id, child_bounds);
            offset += child_size + flex.gap;
        }
    }
}
```

### GPU Renderer

```rust
pub struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    pipeline: RenderPipeline,
    atlas: TextureAtlas,
}

impl GpuRenderer {
    pub fn render(&mut self, scene: &SceneGraph) {
        let output = self.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&Default::default());
        
        let mut encoder = self.device.create_command_encoder(&Default::default());
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                ..Default::default()
            });
            
            // Render scene nodes
            self.render_node(&mut render_pass, scene, scene.root);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
    
    fn render_node(&self, pass: &mut RenderPass, scene: &SceneGraph, node_id: NodeId) {
        let node = scene.nodes.get(&node_id).unwrap();
        
        if !node.semantics.visible {
            return;
        }
        
        let bounds = node.layout.bounds.unwrap();
        
        // Render background
        if let Some(bg) = &node.visual_hints.background {
            self.render_background(pass, bounds, bg);
        }
        
        // Render content
        match &node.node_type {
            NodeType::Text { content } => {
                self.render_text(pass, bounds, content);
            }
            NodeType::Image { source } => {
                self.render_image(pass, bounds, source);
            }
            _ => {}
        }
        
        // Render children
        for child_id in &node.children {
            self.render_node(pass, scene, *child_id);
        }
    }
}
```

### Axiom Integration

```rust
pub struct VisualAxiomBridge {
    axiom: AxiomClient,
    scene: SceneGraph,
}

impl VisualAxiomBridge {
    pub async fn process_action(&mut self, action: SemanticAction) -> Result<(), VisualError> {
        // Submit action to Axiom
        let proposal = Proposal {
            entry_type: EntryType::UiAction,
            payload: UiActionPayload {
                scene_version: self.scene.version,
                action: action.clone(),
            }.into(),
            ..Default::default()
        };
        
        let result = self.axiom.submit(proposal).await?;
        
        // Apply changes based on action
        let changes = self.derive_changes(&action);
        let diff = self.scene.update(changes);
        
        // Commit new scene state
        let proposal = Proposal {
            entry_type: EntryType::SceneUpdate,
            payload: SceneUpdatePayload {
                from_version: diff.from_version,
                to_version: diff.to_version,
                diff: diff.clone(),
            }.into(),
            ..Default::default()
        };
        
        self.axiom.submit(proposal).await?;
        
        Ok(())
    }
}
```

---

## Implementation Steps

### Week 1-3: Scene Graph

1. Define node structure
2. Implement tree operations
3. Create layout engine
4. Add scene diffing
5. Write tests

### Week 4-6: Input & Actions

1. Implement event processing
2. Add hit testing
3. Create focus management
4. Generate semantic actions
5. Axiom integration

### Week 7-10: Renderer

1. Setup wgpu
2. Create render pipeline
3. Implement text rendering
4. Add image support
5. Performance optimization

### Week 11-13: Compositor

1. Implement window management
2. Add layer compositing
3. Damage tracking
4. Multi-window support

### Week 14-16: Integration & Polish

1. Full integration testing
2. Performance tuning
3. Visual polish
4. Documentation

---

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| Scene graph is deterministic | Replay test |
| Layout is correct | Visual comparison |
| 60fps rendering | Performance test |
| Actions flow through Axiom | Audit test |
| GPU acceleration works | Performance test |

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Layout computation | < 1ms |
| Render time | < 8ms |
| Input latency | < 16ms |
| Memory usage | < 100MB baseline |

---

*[← Phase 7](07-phase-bare-metal.md) | [Roadmap →](00-roadmap.md)*
