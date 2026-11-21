class Widget {
public:
    int width;
    char *title;
    bool visible;

    int area() const { return width; }
};

struct Point {
    int x;
    int y;
};
