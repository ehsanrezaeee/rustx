use rustx_core::{Kind, Node, Style};
use rustx_engine::{cpu::CpuRenderer, Engine};

#[test]
fn opacity_composites_the_group_once_and_border_stays_inside() {
    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(
                Style::column()
                    .width(40.)
                    .height(40.)
                    .background(0xffffff)
                    .opacity(0.5),
            )
            .children(vec![Node::new(Kind::Div).style(
                Style::column().width(40.).height(40.).background(0x000000),
            )])
    });
    engine.frame(40., 40., 1.);
    let mut cpu = CpuRenderer::new(40, 40);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 40, 40, 1.);
    let pixel = cpu.pixmap.pixel(20, 20).unwrap();
    // The opaque black child hides its white parent before group compositing.
    assert!((5..=6).contains(&pixel.red()));
    assert!((7..=9).contains(&pixel.green()));
    assert!((11..=13).contains(&pixel.blue()));

    let mut engine = Engine::new(|_| {
        Node::new(Kind::Div)
            .style(Style::column().padding(10.))
            .children(vec![Node::new(Kind::Div).style(
                Style::column().width(20.).height(20.).border(4., 0xffffff),
            )])
    });
    engine.frame(40., 40., 1.);
    cpu.draw(engine.boxes.as_ref().unwrap(), &mut engine.text, 40, 40, 1.);
    assert_eq!(cpu.pixmap.pixel(9, 20).unwrap().red(), 11);
    assert_eq!(cpu.pixmap.pixel(11, 20).unwrap().red(), 255);
}
