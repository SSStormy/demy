#ifndef DEMY_H
#define DEMY_H

struct demy_timeline;
struct demy_track;
struct demy_node;
struct demy_node_iterator;

enum interp_t
{
    DEMY_E_INTERP_NONE = 0,
    DEMY_E_INTERP_LINEAR = 1
};

demy_timeline* demy_tl_new();

void demy_tl_free(demy_timeline* tl);

const demy_track* demy_tl_track_get(demy_timeline* tl);

bool demy_tl_track_del(demy_timeline *tl, const char *name);

bool demy_tr_add_node(demy_track *track, unsigned int time, double value, interp_t interp);

bool demy_tr_del_node(demy_track *track, unsigned int time);

const demy_node* demy_tr_get_node(demy_track *track, unsigned int time);

demy_node_iterator* demy_tr_iter_start(demy_track *track);

void demy_tr_iter_next(demy_node_iterator **ptr_to_iter);

void demy_tr_iter_end(demy_node_iterator *iter);

const demy_node* demy_tr_iter_get(demy_node_iterator *iter);

bool demy_node_update_at(demy_track *track, unsigned int time, const demy_node *node);

demy_node* demy_node_clone(const demy_node *node);

demy_node* demy_node_new(unsigned int time, double value, interp_t interp);

void demy_node_free(demy_node *node);

#endif // DEMY_H
